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
use oxc::allocator::{Allocator, Dummy, FromIn, Vec as AstVec};
use oxc::ast::ast::{
	ImportDeclaration, ImportDeclarationSpecifier, ImportOrExportKind, Program, Statement,
};
use oxc::ast::{AstBuilder, NONE};
use oxc::span::{Atom, Span};

pub const SET_INIT_SIGIL: &str = "$si$";
pub const RUN_INIT_SIGIL: &str = "$ri$";
pub const APPLY_CLASS_SIGIL: &str = "$acd$";
pub const MODULE_NAME: &str = "@moggie/runtime/polyfill";

pub fn inject_polyfill_import<'b, 'a: 'b>(alloc: &'a Allocator, program: &'b mut Program<'a>) {
	let existing_import = find_root_imports(&program.body)
		.filter(|import| import.source.value.eq_ignore_ascii_case(MODULE_NAME))
		.count();

	if existing_import == 0 {
		let builder = AstBuilder::new(alloc);
		let import = builder.alloc_import_declaration(
			Span::dummy(alloc),
			Some(import_set(alloc, &builder)),
			builder.string_literal(Span::dummy(alloc), Atom::from_in(MODULE_NAME, alloc), None),
			None,
			NONE,
			ImportOrExportKind::Value,
		);
		let idx = insertion_index(&program.body);
		if let Some(idx) = idx {
			program
				.body
				.insert(idx, Statement::ImportDeclaration(import));
		} else {
			program.body.push(Statement::ImportDeclaration(import));
		}
	}
}

fn import<'b, 'a: 'b>(
	alloc: &'a Allocator,
	builder: &'b AstBuilder<'a>,
	name: &str,
) -> ImportDeclarationSpecifier<'a> {
	builder.import_declaration_specifier_import_specifier(
		Span::dummy(alloc),
		builder.module_export_name_identifier_name(Span::dummy(alloc), Atom::from_in(name, alloc)),
		builder.binding_identifier(Span::dummy(alloc), Atom::from_in(name, alloc)),
		ImportOrExportKind::Value,
	)
}

pub fn import_set<'b, 'a: 'b>(
	alloc: &'a Allocator,
	builder: &'b AstBuilder<'a>,
) -> AstVec<'a, ImportDeclarationSpecifier<'a>> {
	let mut vec = AstVec::new_in(alloc);
	vec.push(import(alloc, builder, SET_INIT_SIGIL));
	vec.push(import(alloc, builder, RUN_INIT_SIGIL));
	vec.push(import(alloc, builder, APPLY_CLASS_SIGIL));
	vec
}

pub fn find_root_imports<'b, 'a: 'b>(
	statements: &'b [Statement<'a>],
) -> impl Iterator<Item = &'b ImportDeclaration<'a>> {
	statements.iter().filter_map(|statement| match statement {
		Statement::ImportDeclaration(import) => Some(import.as_ref()),
		_ => None,
	})
}

/// Scan the start of a program body to find the first index that is not an import declaration,
/// in order to insert the polyfill import after any others
///
/// A return value of `None` indicates that the polyfill import should be appended to the end of the body,
/// any other value indicates the index where the polyfill import should be inserted.
pub fn insertion_index<'b, 'a: 'b>(statements: &'b [Statement<'a>]) -> Option<usize> {
	for (idx, statement) in statements.iter().enumerate() {
		if !matches!(statement, Statement::ImportDeclaration(_)) {
			return Some(idx);
		}
	}
	None
}
