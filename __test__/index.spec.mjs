import test from "ava"

import { ModuleTransformer } from "../index.js"

function removeExcess(value) {
	return value.replaceAll(/export {};\s*/g, "").trim()
}

test("it transforms typescript modules", t => {
	const expected = `class Test {
\tconstructor(value) {
\t\tthis.value = value;
\t}
}`

	const transformer = new ModuleTransformer()
	const { code } = transformer.transform(
		"test.ts",
		`class Test { constructor(private value: string) {} }`,
	)
	t.is(removeExcess(code), expected)
})

test("it imports the class decorator polyfill from the runtime module", t => {
	const input = `import { foo } from 'bar';
@foo
class Test {
\tconstructor(private value: string) {}
}`

	const expected = `import { foo } from 'bar';
import { $acd$ } from '@moggie/runtime/polyfill';
const Test = $acd$(class Test {
\tconstructor(value) {
\t\tthis.value = value;
\t}
}, 'Test', [foo]);`

	const transformer = new ModuleTransformer()
	const { code } = transformer.transform("test.ts", input)

	t.is(removeExcess(code), expected)
})
