import type { ReactNode } from "react";
import { Check } from "lucide-react";

// A large glass tile in a module's colors: the icon oversized and cropped by the top-right corner,
// a label on top and the main value in big type at the bottom.
export function GlassTile({
  tint,
  icon,
  label,
  value,
  caption,
  action,
  checked,
  onCheck,
  children,
  className = "",
}: {
  tint: string;
  icon: string | ReactNode;
  label: ReactNode;
  value?: ReactNode;
  caption?: ReactNode;
  action?: ReactNode;
  /** When set, the tile shows a checkbox next to its label. */
  checked?: boolean;
  onCheck?: (value: boolean) => void;
  children?: ReactNode;
  className?: string;
}) {
  return (
    <div
      data-tint={tint}
      className={`glass relative flex min-h-[172px] min-w-0 flex-col overflow-hidden rounded-[22px] bg-[radial-gradient(75%_95%_at_100%_0%,color-mix(in_srgb,var(--glow)_80%,transparent),transparent_75%),radial-gradient(70%_90%_at_90%_100%,color-mix(in_srgb,var(--tint)_32%,transparent),transparent_75%)] p-5 ${className}`}
    >
      {typeof icon === "string" ? (
        <img
          src={icon}
          alt=""
          draggable={false}
          className="pointer-events-none absolute -top-7 -right-7 size-[132px] rotate-[-8deg] object-contain drop-shadow-[0_12px_24px_rgb(0_0_0/0.35)]"
        />
      ) : (
        <div className="pointer-events-none absolute -top-5 -right-5 grid size-[112px] rotate-[-8deg] place-items-center rounded-[30px] border border-white/30 bg-[linear-gradient(160deg,var(--tint),var(--glow))] text-white shadow-[inset_0_2px_0_rgb(255_255_255/0.35),0_12px_24px_rgb(0_0_0/0.35)] [&_svg]:size-12">
          {icon}
        </div>
      )}
      <div className="relative flex items-center gap-2.5 pr-24 text-[13px] font-medium text-ink-2">
        {onCheck && (
          <button
            role="checkbox"
            aria-checked={!!checked}
            onClick={() => onCheck(!checked)}
            className={`grid size-[22px] shrink-0 place-items-center rounded-md border transition ${checked ? "border-white/50 bg-white/25" : "border-white/35 bg-white/5 hover:bg-white/12"}`}
          >
            {checked && <Check className="size-3.5 text-white" strokeWidth={3} />}
          </button>
        )}
        {label}
      </div>
      {children}
      <div className="relative mt-auto pt-8">
        {value && <div className="max-w-[85%] text-[23px] leading-tight font-semibold tracking-tight">{value}</div>}
        <div className="mt-1 flex min-h-8 items-end justify-between gap-3">
          <span className="flex min-w-0 items-center gap-1.5 truncate text-[12.5px] text-ink-2">{caption}</span>
          {action}
        </div>
      </div>
    </div>
  );
}

export function TileButton({ children, onClick }: { children: ReactNode; onClick: () => void }) {
  return (
    <button onClick={onClick} className="glass h-8 shrink-0 rounded-lg px-3.5 text-[12.5px] font-semibold hover:bg-white/20">
      {children}
    </button>
  );
}

// The round glowing call to action (Analisar, Limpar), in the colors of the surrounding tint.
export function RoundAction({ children, onClick, disabled }: { children: ReactNode; onClick: () => void; disabled?: boolean }) {
  return (
    <button
      onClick={onClick}
      disabled={disabled}
      className="relative grid size-[84px] place-items-center rounded-full border border-white/35 bg-[radial-gradient(circle_at_50%_30%,var(--tint),var(--glow)_70%)] text-[14px] font-semibold text-white shadow-[inset_0_2px_0_rgb(255_255_255/0.35),0_0_40px_-4px_var(--glow),0_12px_30px_-10px_rgb(0_0_0/0.5)] transition hover:scale-[1.04] active:scale-[0.98] disabled:pointer-events-none disabled:opacity-45"
    >
      {children}
    </button>
  );
}
