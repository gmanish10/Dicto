import { useEffect } from "react";
import { Link, NavLink, Outlet, useLocation, useNavigate } from "react-router-dom";
import { listen } from "@tauri-apps/api/event";
import { api } from "./lib/ipc";
import { useSettings } from "./hooks/useSettings";

function NavItem({ to, label }: { to: string; label: string }) {
  return (
    <NavLink
      to={to}
      className={({ isActive }) =>
        `block rounded-md px-3 py-2 text-sm font-medium ${
          isActive
            ? "bg-accent text-white"
            : "text-ink-700 hover:bg-ink-100 dark:text-ink-200 dark:hover:bg-ink-700"
        }`
      }
    >
      {label}
    </NavLink>
  );
}

export default function App() {
  const navigate = useNavigate();
  const location = useLocation();
  const { settings, loading } = useSettings();

  // Redirect to onboarding if it hasn't been completed.
  useEffect(() => {
    if (!loading && settings && !settings.onboarding_completed && location.pathname !== "/onboarding") {
      navigate("/onboarding", { replace: true });
    }
  }, [loading, settings, location.pathname, navigate]);

  // Listen for "nav:goto" events fired from the menubar.
  useEffect(() => {
    const unlisten = listen<string>("nav:goto", (event) => {
      navigate(event.payload);
    });
    return () => {
      void unlisten.then((fn) => fn());
    };
  }, [navigate]);

  if (loading || !settings) {
    return (
      <div className="flex h-full items-center justify-center text-ink-400">Loading…</div>
    );
  }

  return (
    <div className="flex h-full">
      <aside className="flex w-52 flex-col border-r border-ink-200 bg-ink-50 p-3 dark:border-ink-700 dark:bg-ink-900">
        <Link to="/settings" className="mb-6 flex items-center gap-2 px-2">
          <span className="text-lg font-semibold tracking-tight">Dicto</span>
          {settings.paused && <span className="pill-yellow">paused</span>}
        </Link>
        <nav className="flex flex-col gap-1">
          <NavItem to="/settings" label="Settings" />
          <NavItem to="/dictionary" label="Dictionary" />
          <NavItem to="/history" label="History" />
          <NavItem to="/about" label="About" />
        </nav>
        <div className="mt-auto px-2 text-xs text-ink-400">
          v{import.meta.env.VITE_APP_VERSION ?? "0.1.0"}
        </div>
        <div className="mt-2 px-2">
          {settings.paused ? (
            <button
              type="button"
              className="btn-secondary w-full text-xs"
              onClick={() => api.resumeDictation()}
            >
              Resume
            </button>
          ) : (
            <button
              type="button"
              className="btn-secondary w-full text-xs"
              onClick={() => api.pauseDictation()}
            >
              Pause Dicto
            </button>
          )}
        </div>
      </aside>
      <main className="flex-1 overflow-y-auto p-6">
        <Outlet />
      </main>
    </div>
  );
}
