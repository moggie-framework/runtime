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
use oxc::allocator::{Allocator, Box as AstBox, CloneIn, Dummy, FromIn, Vec as AstVec};
use oxc::ast::ast::{
	BindingIdentifier, BindingPatternKind, Expression, VariableDeclaration, VariableDeclarationKind,
};
use oxc::ast::{AstBuilder, NONE};
use oxc::span::{Atom, Span};

pub fn expr_string<'b, 'a: 'b>(alloc: &'a Allocator, value: &'b str) -> Expression<'a> {
	let builder = AstBuilder::new(alloc);
	builder.expression_string_literal(Span::dummy(alloc), Atom::from_in(value, alloc), None)
}

pub fn expr_undefined<'b, 'a: 'b>(alloc: &'a Allocator) -> Expression<'a> {
	let builder = AstBuilder::new(alloc);
	builder.expression_identifier(Span::dummy(alloc), "undefined")
}

pub fn decl_variable<'b, 'a: 'b>(
	alloc: &'a Allocator,
	name: BindingIdentifier,
	expr: Expression<'a>,
) -> VariableDeclaration<'a> {
	let builder = AstBuilder::new(alloc);
	builder.variable_declaration(
		Span::dummy(alloc),
		VariableDeclarationKind::Const,
		AstVec::from_array_in(
			[builder.variable_declarator(
				Span::dummy(alloc),
				VariableDeclarationKind::Const,
				builder.binding_pattern(
					BindingPatternKind::BindingIdentifier(AstBox::new_in(
						name.clone_in(alloc),
						alloc,
					)),
					NONE,
					false,
				),
				Some(expr),
				false,
			)],
			alloc,
		),
		false,
	)
}
