import { forwardRef, type ButtonHTMLAttributes, type HTMLAttributes, type ReactNode } from "react";
import { cn } from "../lib/utils";

export function Panel({ className, ...props }: HTMLAttributes<HTMLDivElement>) {
  return <section className={cn("border border-zinc-800 bg-zinc-950/80", className)} {...props} />;
}

export const Button = forwardRef<
  HTMLButtonElement,
  ButtonHTMLAttributes<HTMLButtonElement> & { variant?: "default" | "ghost" | "danger" }
>(function Button({
  className,
  variant = "default",
  ...props
}, ref) {
  return (
    <button
      ref={ref}
      className={cn(
        "inline-flex h-9 items-center justify-center gap-2 rounded-md px-3 text-sm font-medium transition",
        "focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-sky-400",
        "disabled:cursor-not-allowed disabled:opacity-45",
        variant === "default" && "bg-zinc-100 text-zinc-950 hover:bg-white",
        variant === "ghost" && "text-zinc-300 hover:bg-zinc-900 hover:text-white",
        variant === "danger" && "bg-red-500 text-white hover:bg-red-400",
        className,
      )}
      {...props}
    />
  );
});

export function Badge({
  tone = "neutral",
  children,
  className,
}: {
  tone?: "neutral" | "green" | "amber" | "red" | "blue";
  children: ReactNode;
  className?: string;
}) {
  return (
    <span
      className={cn(
        "inline-flex h-6 items-center rounded-full border px-2 text-[11px] font-medium",
        tone === "neutral" && "border-zinc-700 bg-zinc-900 text-zinc-300",
        tone === "green" && "border-emerald-700 bg-emerald-950 text-emerald-300",
        tone === "amber" && "border-amber-700 bg-amber-950 text-amber-300",
        tone === "red" && "border-red-700 bg-red-950 text-red-300",
        tone === "blue" && "border-sky-700 bg-sky-950 text-sky-300",
        className,
      )}
    >
      {children}
    </span>
  );
}

export function Kbd({ children }: { children: ReactNode }) {
  return (
    <kbd className="inline-flex h-5 min-w-5 items-center justify-center rounded border border-zinc-700 bg-zinc-900 px-1 text-[10px] text-zinc-300">
      {children}
    </kbd>
  );
}
