import { useState } from "react";
import { Link, NavLink, Outlet } from "react-router-dom";
import {
  Home,
  Target,
  ListOrdered,
  FlaskConical,
  Github,
  Menu,
  User,
  X,
} from "lucide-react";
import { cn } from "@/lib/cn";

function NavItem({
  to,
  icon: Icon,
  label,
  end,
  onNavigate,
}: {
  to: string;
  icon: typeof Target;
  label: string;
  end?: boolean;
  onNavigate?: () => void;
}) {
  return (
    <NavLink
      to={to}
      end={end}
      onClick={onNavigate}
      className={({ isActive }) =>
        cn(
          "flex items-center gap-3 rounded-md px-3 py-2 text-sm transition-colors",
          "text-ink-muted hover:bg-bg-2 hover:text-ink-strong",
          isActive && "bg-bg-2 text-ink-strong",
        )
      }
    >
      <Icon className="size-4" />
      <span>{label}</span>
    </NavLink>
  );
}

/** Sidebar contents, shared by the desktop rail and the mobile drawer. */
function SidebarBody({ onNavigate }: { onNavigate?: () => void }) {
  return (
    <>
      <Link
        to="/"
        onClick={onNavigate}
        className="flex items-center gap-2 px-4 py-5 text-ink-strong"
      >
        <div className="grid size-8 place-items-center rounded-md bg-accent/15 text-accent">
          <Target className="size-5" />
        </div>
        <span className="font-mono text-lg tracking-tight">gomoku</span>
        <span className="text-xs text-ink-muted">dev</span>
      </Link>

      <nav className="flex flex-col gap-1 px-3">
        <NavItem to="/" icon={Home} label="Home" end onNavigate={onNavigate} />
        <NavItem to="/games" icon={ListOrdered} label="Games" onNavigate={onNavigate} />
        {/* <NavItem to="/profile" icon={User} label="Profile" onNavigate={onNavigate} /> */}
        {/* <NavItem to="/lab" icon={FlaskConical} label="Lab" onNavigate={onNavigate} /> */}
      </nav>

      <div className="absolute bottom-0 w-full border-t border-border px-4 py-3 text-xs text-ink-muted">
        <a
          href="https://github.com/42-Course/Gomoku"
          className="flex items-center gap-2 hover:text-ink-strong"
          target="_blank"
          rel="noreferrer"
        >
          <Github className="size-4" />
          local-first · offline ok
        </a>
      </div>
    </>
  );
}

export function Shell() {
  // The mobile drawer closes on every nav link, the backdrop, and the close
  // button, so it can't survive a navigation — no route-change effect needed.
  const [open, setOpen] = useState(false);

  return (
    <div className="flex min-h-screen bg-bg-0 text-ink">
      {/* Desktop rail — persistent from md up. */}
      <aside className="relative hidden w-56 shrink-0 border-r border-border bg-bg-1 md:block">
        <SidebarBody />
      </aside>

      {/* Mobile drawer + backdrop. */}
      {open && (
        <div className="fixed inset-0 z-50 md:hidden" role="dialog" aria-modal="true">
          <div
            className="absolute inset-0 bg-bg-0/70"
            onClick={() => setOpen(false)}
          />
          <aside className="absolute inset-y-0 left-0 w-64 border-r border-border bg-bg-1 shadow-2xl">
            <button
              onClick={() => setOpen(false)}
              aria-label="Close menu"
              className="absolute right-2 top-2 z-10 rounded-md p-1.5 text-ink-muted hover:bg-bg-2 hover:text-ink-strong"
            >
              <X className="size-4" />
            </button>
            <SidebarBody onNavigate={() => setOpen(false)} />
          </aside>
        </div>
      )}

      <div className="flex min-w-0 flex-1 flex-col">
        {/* Mobile top bar with the menu toggle — hidden from md up. */}
        <header className="flex items-center gap-3 border-b border-border bg-bg-1 px-4 py-2.5 md:hidden">
          <button
            onClick={() => setOpen(true)}
            aria-label="Open menu"
            className="rounded-md p-1.5 text-ink-muted hover:bg-bg-2 hover:text-ink-strong"
          >
            <Menu className="size-5" />
          </button>
          <Link to="/" className="flex items-center gap-2 text-ink-strong">
            <Target className="size-4 text-accent" />
            <span className="font-mono text-base tracking-tight">gomoku</span>
          </Link>
        </header>

        <main className="min-w-0 flex-1">
          <Outlet />
        </main>
      </div>
    </div>
  );
}
