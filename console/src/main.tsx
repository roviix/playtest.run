import { render } from "preact";

import { App } from "./app";
import "./style.css";
import "../../ui/workspace.css";
import "./workspace.css";
import "./docs.css";
import "../../ui/dialog.css";
import "./auth.css";

const root = document.getElementById("app");
if (root) render(<App />, root);
