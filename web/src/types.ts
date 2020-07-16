import { editor } from "monaco-editor";

export type NoConflict = {
    type: "noConflict";
};

export type Merged = {
    type: "merged";
    merged: string;
    oid: Oid;
    rev: Oid;
};

export type EditSubmitResponse = Merged | NoConflict;

export type Oid = string;

export type PreviewMarkdown = {
    readonly markdown: string;
};

export type RenderedMarkdown = {
    readonly rendered: string;
};

export type EditSubmit = {
    readonly commitMsg: string;
    readonly markdown: string;
    readonly oid?: Oid;
    readonly rev: Oid;
};

export type ArticleInfo = {
    readonly markdown: string;
    readonly oid?: Oid;
    readonly rev: Oid;
};

export type Model = {
    articleInfo: ArticleInfo;
    readonly title: string;
    readonly editor: editor.ICodeEditor;
    diffEditor?: editor.IDiffEditor;
    readonly tabs: Map<HTMLElement, HTMLElement>;
    activeEditor: editor.ICodeEditor;
};

