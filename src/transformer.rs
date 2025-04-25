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
use napi::Result;
use oxc::allocator::{Allocator, CloneIn, Dummy};
use oxc::allocator::{FromIn, Vec as AstVec};
use oxc::ast::ast::{
	Argument, Class, ClassElement, Decorator, Expression, MethodDefinitionKind, Program, Statement,
	TSType, TSTypeParameterInstantiation,
};
use oxc::ast::{AstBuilder, NONE};
use oxc::codegen::{Codegen, CodegenOptions};
use oxc::parser::{Parser, ParserReturn};
use oxc::semantic::SemanticBuilder;
use oxc::span::{Atom, SourceType, Span};
use oxc::transformer::{EnvOptions, TransformOptions, Transformer};
use std::fmt::Debug;
use std::path::Path;

mod decorators;
mod polyfill;
mod utils;

#[napi]
pub struct ModuleTransformer {
	alloc: Allocator,
}

#[napi]
pub struct TransformResult {
	pub code: String,
}

pub struct ResultWrap<'a> {
	r: ParserReturn<'a>,
}

impl Debug for ResultWrap<'_> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("ParserResult")
			.field(
				"irregular_whitespaces",
				&self.r.irregular_whitespaces.iter().collect::<Vec<&Span>>(),
			)
			.field("program", &self.r.program)
			.field("modules", &self.r.module_record)
			.finish()
	}
}

#[napi]
impl ModuleTransformer {
	#[napi(constructor)]
	pub fn new() -> Result<Self> {
		Ok(ModuleTransformer {
			alloc: Allocator::default(),
		})
	}

	#[napi]
	pub fn recycle(&mut self) {
		let previous = std::mem::take(&mut self.alloc);
		drop(previous);
	}

	#[napi]
	pub fn transform(&mut self, file_name: String, code: String) -> Result<TransformResult> {
		let source_path = Path::new(&file_name);
		let mut source_dir = source_path.to_path_buf().clone();
		source_dir.pop();

		let source_type = SourceType::from_path(&file_name)
			.map_err(|err| napi::Error::from_reason(format!("{err}")))?;
		let mut result = Parser::new(&self.alloc, &code, source_type).parse();

		if result.panicked {
			return Err(napi::Error::from_reason(
				format!(
					"Parsing failed; {}",
					result.errors.iter().fold(String::new(), |mut acc, err| {
						acc.push_str(&format!("[{:?}] {}\n", err.labels, err.message));
						acc
					})
				)
				.trim(),
			));
		}

		result = transform_module(result, &self.alloc);
		polyfill::inject_polyfill_import(&self.alloc, &mut result.program);
		decorators::transform_class_decorators(&self.alloc, &mut result.program);

		let mut program = result.program;
		let result = SemanticBuilder::new()
			.with_excess_capacity(2.0)
			.with_check_syntax_error(true)
			.build(&program);

		let _ = Transformer::new(
			&self.alloc,
			&source_path,
			&TransformOptions {
				cwd: source_dir,
				env: EnvOptions::from_target("es2022").unwrap(),
				..Default::default()
			},
		)
		.build_with_scoping(result.semantic.into_scoping(), &mut program);

		let output = Codegen::new()
			.with_options(CodegenOptions {
				single_quote: true,
				..Default::default()
			})
			.build(&program);

		self.recycle();
		Ok(TransformResult { code: output.code })
	}
}

fn transform_module<'a>(mut result: ParserReturn<'a>, alloc: &'a Allocator) -> ParserReturn<'a> {
	let entries = result.program.body.len();
	let mut should_add_import = false;
	let body = std::mem::replace(
		&mut result.program.body,
		oxc::allocator::Vec::with_capacity_in(entries, alloc),
	);

	for mut statement in body.into_iter() {
		let statement = match statement {
			Statement::ClassDeclaration(ref mut class_decl) => {
				if requires_decorator(class_decl) {
					let decorator = extract_injectable_class_constructor(alloc, class_decl);
					if let Some(decorator) = decorator {
						class_decl.decorators.push(decorator);
					}
				}
				Statement::ClassDeclaration(class_decl.clone_in(alloc))
			}
			other => other,
		};
		result.program.body.push(statement);
	}
	result
}

fn requires_decorator<'a, 'b: 'a>(class: &'b Class<'a>) -> bool {
	!class.decorators.iter().any(
		|decorator| matches!(decorator.name(), Some(name) if name.eq_ignore_ascii_case("depends")),
	)
}

fn extract_injectable_class_constructor<'b, 'a: 'b>(
	alloc: &'a Allocator,
	class: &'b mut Class<'a>,
) -> Option<Decorator<'a>> {
	let mut constructor = class.body.body.iter_mut().find(|part|
        matches!(part, ClassElement::MethodDefinition(method) if method.kind == MethodDefinitionKind::Constructor)
    );

	let mut injects = Vec::new();
	let mut expected = 0;

	if let Some(ClassElement::MethodDefinition(ref mut constructor)) = &mut constructor {
		expected = constructor.value.params.items.len();
		constructor.value.params.items.iter_mut().for_each(|item| {
			let mut had_alias = false;

			let decorators = std::mem::replace(&mut item.decorators, AstVec::new_in(alloc));
			item.decorators = AstVec::from_iter_in(
				decorators.into_iter().filter_map(|dec| {
					if let Expression::CallExpression(call) = &dec.expression {
						match &call.callee {
							Expression::Identifier(ident)
								if ident.name.eq_ignore_ascii_case("alias") =>
							{
								match call.arguments.as_slice() {
									[Argument::StringLiteral(lit)] => {
										injects.push(lit.value.as_str().to_string());
										had_alias = true;
										return None;
									}
									_ => {}
								}
							}
							_ => {}
						}
					}
					Some(dec)
				}),
				alloc,
			);

			if !had_alias {
				if let Some(annotation) = &item.pattern.type_annotation {
					match annotation.type_annotation {
						TSType::TSTypeReference(ref ref_type) => {
							let name = format!("{}", ref_type.type_name);
							injects.push(name);
						}
						_ => {}
					}
				}
			}
		});
	}

	if injects.is_empty() || injects.len() != expected {
		None
	} else {
		let builder = AstBuilder::new(alloc);
		Some(builder.decorator(
			Span::dummy(alloc),
			builder.expression_call(
				Span::dummy(alloc),
				builder.expression_identifier(Span::dummy(alloc), "depends"),
				NONE,
				oxc::allocator::Vec::from_iter_in(
					injects.iter().map(|item| {
						Argument::StringLiteral(builder.alloc_string_literal(
							Span::dummy(alloc),
							Atom::from_in(item.as_str(), alloc),
							None,
						))
					}),
					&alloc,
				),
				false,
			),
		))
	}
}
