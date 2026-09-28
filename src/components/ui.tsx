import type { ButtonHTMLAttributes, ReactNode } from "react";
import { Loader2 } from "lucide-react";

type Variant = "primary" | "secondary" | "ghost" | "danger";

const variants: Record<Variant, string> = {
  primary: "bg-accent text-accent-ink hover:brightness-105 active:brightness-95 shadow-sm",
  secondary: "bg-surface-2 text-ink border border-line hover:bg-surface-3",
  ghost: "text-ink-2 hover:bg-surface-2 hover:text-ink",
  danger: "bg-danger text-white hover:brightness-105 active:brightness-95 shadow-sm",
};

export function Button({
  variant = "secondary",
  size = "md",
  busy,
  icon,
  children,
  className = "",
  ...props
}: ButtonHTMLAttributes<HTMLButtonElement> & { variant?: Variant; size?: "sm" | "md"; busy?: boolean; icon?: ReactNode }) {
  const sizing = size === "sm" ? "h-7 px-2.5 text-[12px] gap-1.5" : "h-8 px-3.5 text-[13px] gap-2";
  return (
    <button
      {...props}
      disabled={props.disabled || busy}
      className={`inline-flex items-center justify-center rounded-md font-medium whitespace-nowrap transition disabled:opacity-45 disabled:pointer-events-none ${sizing} ${variants[variant]} ${className}`}
    >
      {busy ? <Loader2 className="size-3.5 animate-spin" /> : icon}
      {children}
    </button>
  );
}

export function PageHeader({ title, subtitle, actions }: { title: string; subtitle?: ReactNode; actions?: ReactNode }) {
  return (
    <header className="drag flex items-end justify-between gap-4 px-8 pt-9 pb-5">
      <div className="min-w-0">
        <h1 className="text-[22px] font-semibold tracking-tight text-ink">{title}</h1>
        {subtitle && <p className="mt-1 text-[13px] text-ink-2 max-w-2xl">{subtitle}</p>}
      </div>
      {actions && <div className="flex shrink-0 items-center gap-2 [-webkit-app-region:no-drag]">{actions}</div>}
    </header>
  );
}

export function Card({ children, className = "" }: { children: ReactNode; className?: string }) {
  return <section className={`rounded-xl border border-line bg-surface ${className}`}>{children}</section>;
}

export function Stat({ label, value, hint }: { label: string; value: ReactNode; hint?: ReactNode }) {
  return (
    <div className="min-w-0">
      <div className="text-[11px] font-medium uppercase tracking-wide text-ink-3">{label}</div>
      <div className="tabular mt-1 text-[20px] font-semibold text-ink truncate">{value}</div>
      {hint && <div className="mt-0.5 text-[12px] text-ink-3">{hint}</div>}
    </div>
  );
}

export function ProgressBar({ value, indeterminate }: { value?: number; indeterminate?: boolean }) {
  return (
    <div className="h-1.5 w-full overflow-hidden rounded-full bg-surface-3">
      {indeterminate ? (
        <div className="h-full w-1/3 rounded-full bg-accent animate-[slide_1.2s_ease-in-out_infinite]" />
      ) : (
        <div className="h-full rounded-full bg-accent transition-[width]" style={{ width: `${Math.min(100, (value ?? 0) * 100)}%` }} />
      )}
    </div>
  );
}

export function EmptyState({ icon, title, children }: { icon: ReactNode; title: string; children?: ReactNode }) {
  return (
    <div className="flex flex-col items-center justify-center px-8 py-16 text-center">
      <div className="mb-4 grid size-14 place-items-center rounded-2xl bg-accent-soft text-accent">{icon}</div>
      <h2 className="text-[15px] font-semibold text-ink">{title}</h2>
      {children && <div className="mt-1.5 max-w-md text-[13px] text-ink-2">{children}</div>}
    </div>
  );
}

export function Badge({ tone = "neutral", children }: { tone?: "neutral" | "safe" | "review" | "danger" | "accent"; children: ReactNode }) {
  const tones = {
    neutral: "bg-surface-3 text-ink-2",
    safe: "bg-[color-mix(in_srgb,var(--safe)_14%,transparent)] text-safe",
    review: "bg-[color-mix(in_srgb,var(--review)_16%,transparent)] text-review",
    danger: "bg-danger-soft text-danger",
    accent: "bg-accent-soft text-accent",
  };
  return <span className={`inline-flex items-center rounded px-1.5 py-0.5 text-[11px] font-medium ${tones[tone]}`}>{children}</span>;
}

export function ErrorNote({ children }: { children: ReactNode }) {
  return <div className="rounded-lg border border-danger/30 bg-danger-soft px-3 py-2 text-[12px] text-danger selectable">{children}</div>;
}
