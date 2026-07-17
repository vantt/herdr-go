// herdr-gateway web entry point (skeleton — real UI lands in slice S5).
import { appVersion } from "./version";

const app = document.getElementById("app");
if (app) {
  app.textContent = `herdr-gateway ${appVersion()} — skeleton`;
}
