import { Link, NavLink, Outlet } from "react-router-dom";
import {
  Home,
  Target,
  ListOrdered,
  FlaskConical,
  Github,
  Play,
  User,
} from "lucide-react";
import { cn } from "@/lib/cn";

function NavItem({
  to,
  icon: Icon,
  label,
  end,
}: {
  to: string;
  icon: typeof Target;
  label: string;
  end?: boolean;
}) {
  return (
    <NavLink
      to={to}
      end={end}
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

export function Shell() {
  return (
    <div className="flex min-h-screen bg-bg-0 text-ink">
      <aside className="relative w-56 shrink-0 border-r border-border bg-bg-1">
        <Link to="/" className="flex items-center gap-2 px-4 py-5 text-ink-strong">
          <div className="grid size-8 place-items-center rounded-md bg-accent/15 text-accent">
            <Target className="size-5" />
          </div>
          <span className="font-mono text-lg tracking-tight">gomoku</span>
          <span className="text-xs text-ink-muted">dev</span>
        </Link>

        <nav className="flex flex-col gap-1 px-3">
          <NavItem to="/" icon={Home} label="Home" end />
          <NavItem to="/play" icon={Play} label="Play" />
          <NavItem to="/games" icon={ListOrdered} label="Games" />
          <NavItem to="/profile" icon={User} label="Profile" />
          <NavItem to="/lab" icon={FlaskConical} label="Lab" />
        </nav>

        <div className="absolute bottom-0 w-56 border-t border-border px-4 py-3 text-xs text-ink-muted">
          <a
            href="https://github.com"
            className="flex items-center gap-2 hover:text-ink-strong"
            target="_blank"
            rel="noreferrer"
          >
            <Github className="size-4" />
            local-first · offline ok
          </a>
        </div>
      </aside>

      <main className="flex-1">
        <Outlet />
      </main>
    </div>
  );
}
