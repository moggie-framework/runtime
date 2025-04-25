declare type Initializer = () => void

export function $si$(target: Object, fns: Initializer[]): void
export function $ri$(): void
export function $acd$<T>(target: T, targetName: string | undefined, decorators: ClassDecorator[]): T
