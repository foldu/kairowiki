import "./css/style.css";
import "./img/logo.svg";
import { $$ } from "./util";

window.addEventListener("load", () => {
    for (const elt of $$(".date")) {
        elt.innerText = new Date(elt.innerText.trim()).toLocaleString();
    }
});
