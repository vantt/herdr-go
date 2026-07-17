import { login as apiLogin } from "../api";

export interface LoginProps {
  onSuccess: () => void;
}

export function renderLogin(root: HTMLElement, props: LoginProps): void {
  root.innerHTML = `
    <div class="view view-login">
      <div class="login-card">
        <div class="brand">
          <span class="brand-mark" aria-hidden="true"></span>
          <h1 class="brand-word">herdr<span class="brand-dot">&middot;</span>gateway</h1>
        </div>
        <p class="login-help">Enter your access token to connect.</p>
        <form id="login-form" novalidate>
          <label class="field-label" for="token">Access token</label>
          <input
            id="token"
            name="token"
            type="password"
            autocomplete="current-password"
            spellcheck="false"
            autocapitalize="off"
            required
            class="field-input"
            placeholder="&bull;&bull;&bull;&bull;&bull;&bull;&bull;&bull;&bull;&bull;&bull;&bull;"
          />
          <button type="submit" class="btn btn-primary btn-block" id="login-submit">
            Connect
          </button>
          <p class="login-error" id="login-error" role="alert" aria-live="polite"></p>
        </form>
      </div>
    </div>
  `;

  const form = root.querySelector<HTMLFormElement>("#login-form")!;
  const input = root.querySelector<HTMLInputElement>("#token")!;
  const button = root.querySelector<HTMLButtonElement>("#login-submit")!;
  const error = root.querySelector<HTMLParagraphElement>("#login-error")!;

  input.focus();

  form.addEventListener("submit", (ev) => {
    ev.preventDefault();
    void handleSubmit();
  });

  async function handleSubmit(): Promise<void> {
    const token = input.value.trim();
    if (!token) return;

    error.textContent = "";
    button.disabled = true;
    button.classList.add("is-loading");
    try {
      const ok = await apiLogin(token);
      if (ok) {
        props.onSuccess();
        return;
      }
      error.textContent = "Access denied. Check your token and try again.";
      input.select();
    } catch {
      error.textContent = "Could not reach the gateway. Try again.";
    } finally {
      button.disabled = false;
      button.classList.remove("is-loading");
    }
  }
}
