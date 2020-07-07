export function $<T extends Element = HTMLElement>(query: string): T {
    const elt = document.querySelector(query);
    if (elt) {
        return elt as T;
    } else {
        throw `Can't find element with selector ${query}`;
    }
}

export function $$(query: string): NodeListOf<HTMLElement> {
    return document.querySelectorAll(query) as NodeListOf<HTMLElement>;
}

export function stripPrefix(s: string, prefix: string): string {
    return s.indexOf(prefix) === 0 ? s.slice(prefix.length) : s;
}

export function $e(
    ty: string,
    attrs: Object = {},
    children: Array<HTMLElement> | string = [],
): HTMLElement {
    const ret = document.createElement(ty);

    Object.assign(ret, attrs);

    if (typeof children === "string") {
        ret.textContent = children;
    } else {
        ret.append(...children);
    }

    return ret;
}
