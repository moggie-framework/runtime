/*
 * Copyright 2025 Weird Boi
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     https://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */
use crate::transformer::polyfill::APPLY_CLASS_SIGIL;
use crate::transformer::utils::{decl_variable, expr_string};
use oxc::allocator::{Allocator, Box as AstBox, CloneIn, Dummy, Vec as AstVec};
use oxc::ast::ast::{
	Argument, ArrayExpressionElement, BindingPatternKind, Class, Declaration,
	ExportDefaultDeclarationKind, Expression, Program, Statement, VariableDeclarationKind,
};
use oxc::ast::{AstBuilder, NONE};
use oxc::span::Span;

pub fn transform_class_decorators<'b, 'a: 'b>(alloc: &'a Allocator, program: &'b mut Program<'a>) {
	for mut statement in &mut program.body {
		match statement {
			Statement::ClassDeclaration(class) => {
				if let Some(id) = class.id.as_ref() {
					if !class.decorators.is_empty() {
						let expr = wrap_class_decl(alloc, class.as_ref().clone_in(alloc));
						let variable_decl =
							AstBox::new_in(decl_variable(alloc, id.clone_in(alloc), expr), alloc);
						*statement = Statement::VariableDeclaration(variable_decl);
					}
				}
			}
			Statement::VariableDeclaration(variable_decl) => {
				for decl in &mut variable_decl.declarations {
					if let Some(Expression::ClassExpression(class)) = &mut decl.init {
						if !class.decorators.is_empty() {
							let expr = wrap_class_decl(alloc, class.as_ref().clone_in(alloc));
							decl.init = Some(expr);
						}
					}
				}
			}
			Statement::ExportNamedDeclaration(named_export) => {
				if let Some(Declaration::ClassDeclaration(ref class)) = &named_export.declaration {
					if !class.decorators.is_empty() {
						if let Expression::ClassExpression(class) =
							wrap_class_decl(alloc, class.as_ref().clone_in(alloc))
						{
							named_export.declaration = Some(Declaration::ClassDeclaration(class));
						}
					}
				}
			}
			Statement::ExportDefaultDeclaration(default_export) => {
				match &default_export.declaration {
					ExportDefaultDeclarationKind::ClassDeclaration(class)
					| ExportDefaultDeclarationKind::ClassExpression(class) => {
						if !class.decorators.is_empty() {
							let class_expr = wrap_class_decl(alloc, class.as_ref().clone_in(alloc));
							default_export.declaration =
								ExportDefaultDeclarationKind::from(class_expr);
						}
					}
					_ => {}
				}
			}
			_ => {}
		}
	}
}

fn wrap_class_decl<'b, 'a: 'b>(alloc: &'a Allocator, mut class: Class<'a>) -> Expression<'a> {
	let builder = AstBuilder::new(alloc);

	if class.decorators.is_empty() {
		return Expression::ClassExpression(AstBox::new_in(class, alloc));
	}

	let class_name_expr = match &class.id {
		Some(id) => expr_string(alloc, id.name.as_str()),
		None => return Expression::ClassExpression(AstBox::new_in(class, alloc)),
	};

	// Convert ordered decorator list directly to argument list while stripping the
	// decorator part. A decorator could be one of a small range of expressions,
	// all of which are valid in the argument value position of a function call.
	let decorators = builder.expression_array(
		Span::dummy(alloc),
		AstVec::from_iter_in(
			std::mem::replace(&mut class.decorators, AstVec::new_in(alloc))
				.into_iter()
				.map(|decorator| ArrayExpressionElement::from(decorator.expression)),
			alloc,
		),
	);

	let mut arguments = AstVec::with_capacity_in(3, alloc);
	arguments.push(Argument::from(Expression::ClassExpression(AstBox::new_in(
		class, alloc,
	))));
	arguments.push(Argument::from(class_name_expr));
	arguments.push(Argument::from(decorators));

	builder.expression_call(
		Span::dummy(alloc),
		builder.expression_identifier(Span::dummy(alloc), APPLY_CLASS_SIGIL),
		NONE,
		arguments,
		false,
	)
}
