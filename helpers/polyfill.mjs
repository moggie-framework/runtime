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

const InitialiserStore = Symbol("[[initialiser]]")

function setInitialisers(target, list) {
	/** @type {Array<Function>} existing initialisers */
	const existing = Reflect.get(target, InitialiserStore) ?? []
	/** @type {Array<Function>} new initialisers */
	const additions = Array.isArray(list) ? list : [list]

	Object.defineProperty(target, InitialiserStore, {
		value: [...existing, ...additions],
		enumerable: false,
		writable: false,
		configurable: false,
	})
}

function runInitialisers() {
	if (this.constructor == null) {
		throw new TypeError("Cannot run initialisers on object that is not a class instance")
	}

	/** @type {Array<Function>} initialisers */
	const initialisers = Reflect.get(this.constructor, InitialiserStore)
	if (initialisers) {
		for (const initializer of initialisers) {
			initializer.call(this)
		}
	}
}

/**
 * @param {Class} hostClass
 * @param {string | undefined} className
 * @param {ClassDecorator[]} decoratorList
 */
function applyAllClassDecorators(hostClass, className, decoratorList) {
	const initialisers = []
	const context = {
		kind: "class",
		name: className,
		addInitializer: initializer => initialisers.push(initializer),
	}

	let result = hostClass
	while (decoratorList.length > 0) {
		const decorator = decoratorList.pop()
		result = decorator(result, context) ?? result
	}

	for (const initializer of initialisers) {
		initializer.call(result)
	}

	return result
}

export { setInitialisers as $si$, runInitialisers as $ri$, applyAllClassDecorators as $acd$ }
