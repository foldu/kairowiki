import * as monaco from "monaco-editor";
import {
    NoConflict,
    Merged,
    EditSubmit,
    Oid,
    PreviewMarkdown,
    EditSubmitResponse,
    ArticleInfo,
    Model,
    RenderedMarkdown,
} from "./types";
import { $, stripPrefix, $e } from "./util";

// @ts-ignore
self.MonacoEnvironment = {
    getWorkerUrl: function (_moduleId: any, label: any) {
        if (label === "json") {
            return "/static/json.worker.bundle.js";
        }
        if (label === "css") {
            return "/static/css.worker.bundle.js";
        }
        if (label === "html") {
            return "/static/html.worker.bundle.js";
        }
        if (label === "typescript" || label === "javascript") {
            return "/static/ts.worker.bundle.js";
        }
        return "/static/editor.worker.bundle.js";
    },
};

function insertImageLink(editor: monaco.editor.ICodeEditor, url: string) {
    const md = `![](${url})`;
    const selection = editor.getSelection();
    const range = new monaco.Range(
        selection.startLineNumber,
        selection.startColumn,
        selection.endLineNumber,
        selection.endColumn,
    );
    const identifier = { major: 1, minor: 1 };
    const insertOp = {
        identifier,
        range,
        text: md,
        forceMoveMarkers: true,
    };

    // TODO: figure out how to move the cursor to ![here](...)
    editor.executeEdits("kairowiki", [insertOp]);
}

// FIXME:
async function getJson<T>(url: string): Promise<T | null> {
    try {
        const res = await fetch(url, { method: "GET" });
        if (res.status !== 200) {
            displayGenericError();
            return null;
        }
        return await res.json();
    } catch (e) {
        console.error(e);
        displayGenericError();
        return null;
    }
}

function displayGenericError() {
    notify("Error", "Looks like something went wrong :^)");
}

enum Method {
    Post = "POST",
    Put = "PUT",
}

// FIXME:
async function sendJson<T>(
    url: string,
    method: Method,
    body: Object,
): Promise<T | null> {
    try {
        const res = await fetch(url, {
            method,
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify(body),
            credentials: "same-origin",
        });

        if (res.status !== 200) {
            displayGenericError();
            return null;
        }

        return await res.json();
    } catch (e) {
        console.error(e);
        displayGenericError();
        return null;
    }
}

function notify(title: string, body: string) {
    const elt = $e("li", {}, [$e("div", {}, title), $e("div", {}, body)]);
    const removeNotification = () => {
        elt.remove();
    };
    elt.onclick = removeNotification;
    setTimeout(removeNotification, 5 * 1000);

    $("#notifications").append(elt);
}

function clearFileList(elt: HTMLInputElement) {
    elt.value = "";
}

function addFileInput(model: Model) {
    const uploadFile = async (evt: InputEvent) => {
        const fileInput = evt.target as HTMLInputElement;
        const listElt = fileInput.closest("li");

        const file = fileInput.files[0];
        const data = new FormData();
        data.append("file", file);
        const resp = await fetch("/storage", {
            method: "PUT",
            credentials: "same-origin",
            body: data,
        });
        if (resp.status !== 200) {
            notify("Error", "Could not upload media, please try again later");
            console.error(resp);
            clearFileList(fileInput);
            return;
        }

        const body = await resp.json();

        fileInput.remove();

        listElt.append(
            $e("a", { href: body.url }, [$e("img", { src: body.url })]),
            $e("button", {
                onclick: () => insertImageLink(model.activeEditor, body.url),
                textContent: "Insert markdown",
            }),
            $e("button", {
                onclick: () => listElt.remove(),
                textContent: "Delete",
            }),
        );
        addFileInput(model);
    };

    const listElt = $e("li", {}, [
        $e("input", {
            type: "file",
            classList: "file-input",
            onchange: uploadFile,
        }),
    ]);

    document.querySelector("#file-list").append(listElt);
}

function switchTo(model: Model, targetButton: HTMLElement) {
    const targetTab = model.tabs.get(targetButton);
    if (!targetTab.classList.contains("hidden")) {
        return false;
    }
    targetButton.classList.add("active");

    targetTab.classList.remove("hidden");

    for (const [button, tab] of model.tabs) {
        if (tab !== targetTab) {
            button.classList.remove("active");
            tab.classList.add("hidden");
        }
    }

    return true;
}

function showDiff(model: Model, text: string) {
    if (model.diffEditor === null) {
        const diffDiv = $("#diff-editor");
        document.querySelector("#editor").classList.add("hidden");
        diffDiv.classList.remove("hidden");
        model.diffEditor = monaco.editor.createDiffEditor(diffDiv);
    }
    model.activeEditor = model.diffEditor.getModifiedEditor();

    const modified = monaco.editor.createModel(text);

    // FIXME: logic error
    model.diffEditor.setModel({
        original: monaco.editor.createModel(model.editor.getValue()),
        modified: modified,
    });
}

window.addEventListener("load", async () => {
    const title = stripPrefix(window.location.pathname, "/edit/");
    const articleInfo = await getJson<ArticleInfo>(
        "/api/article_info/" + title,
    );

    if (!articleInfo) return;

    const editor = monaco.editor.create($("#editor"), {
        value: articleInfo.markdown,
        language: "markdown",
        minimap: {
            enabled: false,
        },
    });

    const tabs = new Map([
        [$("#edit-button"), $("#editor-tab")],
        [$("#preview-button"), $("#preview-tab")],
    ]);

    const model: Model = {
        editor,
        tabs,
        diffEditor: null,
        articleInfo,
        title,
        activeEditor: editor,
    };

    addFileInput(model);

    $("#edit-button").addEventListener("click", (evt) => {
        switchTo(model, evt.target as HTMLElement);
    });

    $("#preview-button").addEventListener("click", async (evt) => {
        const needsRender = switchTo(model, evt.target as HTMLElement);
        if (needsRender) {
            const article = document.querySelector("#preview-tab > article");
            article.innerHTML = "Rendering preview";
            const response = await sendJson<RenderedMarkdown>(
                "/api/preview",
                Method.Put,
                { markdown: model.activeEditor.getValue() },
            );

            if (!response) return;

            article.innerHTML = response.rendered;
        }
    });

    $("#save-button").addEventListener("click", async () => {
        const body: EditSubmit = {
            markdown: model.activeEditor.getValue(),
            oid: model.articleInfo.oid,
            rev: model.articleInfo.rev,
            commitMsg: $<HTMLInputElement>("#commit-msg").value,
        };
        const resp = await sendJson<EditSubmitResponse>(
            "/api/edit/" + model.title,
            Method.Put,
            body,
        );

        if (resp) {
            switch (resp.type) {
                case "noConflict":
                    window.location.href = "/wiki/" + model.title;
                    break;
                case "merged":
                    model.articleInfo = {
                        ...model.articleInfo,
                        oid: body.oid,
                        rev: body.rev,
                    };
                    notify("Merge Conflict", "Your changes were auto-merged");
                    showDiff(model, resp.merged);
                    break;
            }
        }
    });
});
