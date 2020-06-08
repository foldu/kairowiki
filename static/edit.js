require.config({
    paths: {
        vs:
            "https://cdnjs.cloudflare.com/ajax/libs/monaco-editor/0.19.2/min/vs",
    },
});

window.MonacoEnvironment = {
    getWorkerUrl: function (workerId, label) {
        return `data:text/javascript;charset=utf-8,${encodeURIComponent(`
          self.MonacoEnvironment = {
            baseUrl: 'https://cdnjs.cloudflare.com/ajax/libs/monaco-editor/0.19.2/min/'
          };
          importScripts('https://cdnjs.cloudflare.com/ajax/libs/monaco-editor/0.19.2/min/vs/base/worker/workerMain.js');`)}`;
    },
};

require(["vs/editor/editor.main"], function () {
    window.editor = monaco.editor.create(document.querySelector("#editor"), {
        value: document.querySelector("#markdown").innerHTML,
        language: "markdown",
        minimap: {
            enabled: false,
        },
    });
});

(() => {
    const fileInput = document.querySelector(".file-input");
    fileInput.addEventListener("change", async () => {
        const file = fileInput.files[0];
        const data = new FormData();
        data.append("file", file);
        const resp = await fetch("/storage", {
            method: "PUT",
            credentials: "same-origin",
            body: data,
        });
        if (resp.status !== 200) {
            console.error(resp);
            return;
        }

        const body = await resp.json();
        document.querySelector(
            ".file-link"
        ).innerHTML = `<a href=${body.url}>${body.url}</a>`;
    });
})();

document.querySelector("#save-button").addEventListener("click", async () => {
    const title = document.querySelector("#title").innerHTML;
    const encodedTitle = encodeURI(title);
    const response = await fetch("/api/edit/" + encodedTitle, {
        method: "PUT",
        credentials: "same-origin",
        headers: {
            "Content-Type": "application/json",
        },
        body: JSON.stringify({
            markdown: window.editor.getValue(),
        }),
    });

    // TODO: handle diff if somebody else commited before
    if (response.status === 200) {
        window.location = "/wiki/" + encodedTitle;
    } else {
        console.error(response);
    }
});

function switchToEdit() {
    document.querySelector("#preview-tab").classList.add("hidden");
    document.querySelector("#editor-tab").classList.remove("hidden");
}

async function switchToPreview() {
    document.querySelector("#editor-tab").classList.add("hidden");
    const previewClassList = document.querySelector("#preview-tab").classList;
    const needsRender = previewClassList.contains("hidden");
    previewClassList.remove("hidden");
    if (needsRender) {
        const article = document.querySelector("#preview-tab > article");
        article.innerHTML = "Rendering preview";
        const response = await fetch("/api/preview", {
            method: "POST",
            credentials: "same-origin",
            headers: {
                "Content-Type": "application/json",
            },
            body: JSON.stringify({
                markdown: editor.getValue(),
            }),
        });

        if (response.status === 200) {
            const json = await response.json();
            console.log(json.rendered);
            article.innerHTML = json.rendered;
        }
    }
}
