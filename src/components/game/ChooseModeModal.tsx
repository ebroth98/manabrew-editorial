import { createPortal } from "react-dom";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { cn } from "@/lib/utils";
import { useState, useEffect, useCallback, useRef } from "react";

interface ChooseModeModalProps {
  /** Human-readable descriptions for each available mode (0-indexed). */
  options: string[];
  /** Minimum number of modes that must be chosen before confirming. */
  minChoices: number;
  /** Maximum number of modes that can be chosen. */
  maxChoices: number;
  onConfirm: (chosenIndices: number[]) => void;
}

export function ChooseModeModal({
  options,
  minChoices,
  maxChoices,
  onConfirm,
}: ChooseModeModalProps) {
  const [selected, setSelected] = useState<Set<number>>(new Set());
  
  // If exactly 1 must be picked and max 1 can be picked, auto-confirm on click.
  const isAutoConfirm = maxChoices === 1 && minChoices === 1;
  const showCheckboxes = maxChoices > 1;
  const canConfirm = selected.size >= minChoices && selected.size <= maxChoices;

  // Reset selection whenever the option list changes (new prompt arrived).
  useEffect(() => {
    setSelected(new Set());
  }, [options]);

  // Auto-focus the dialog container for keyboard accessibility.
  const dialogRef = useRef<HTMLDivElement>(null);
  useEffect(() => {
    dialogRef.current?.focus();
  }, [options]);

  const handleConfirm = useCallback(() => {
    onConfirm([...selected].sort((a, b) => a - b));
  }, [selected, onConfirm]);

  function toggleOption(idx: number) {
    if (isAutoConfirm) {
      onConfirm([idx]);
      return;
    }
    
    setSelected((prev) => {
      const next = new Set(prev);
      if (next.has(idx)) {
        next.delete(idx);
      } else {
        // If it's a single choice (up to 1), clicking a new one replaces the old one
        if (maxChoices === 1) {
          return new Set([idx]);
        }
        if (next.size >= maxChoices) return prev;
        next.add(idx);
      }
      return next;
    });
  }

  // Keyboard: Enter confirms; Escape is intentionally blocked.
  useEffect(() => {
    function handleKey(e: KeyboardEvent) {
      if (e.key === "Enter" && canConfirm && !isAutoConfirm) {
        e.preventDefault();
        handleConfirm();
      }
    }
    window.addEventListener("keydown", handleKey);
    return () => window.removeEventListener("keydown", handleKey);
  }, [canConfirm, isAutoConfirm, handleConfirm]);

  const subtitle =
    minChoices === maxChoices
      ? `Choose ${minChoices} mode${minChoices !== 1 ? "s" : ""}`
      : `Choose ${minChoices}–${maxChoices} modes`;

  return createPortal(
    <div
      className="fixed inset-0 z-[9000] flex items-center justify-center bg-black/60 backdrop-blur-sm"
      role="dialog"
      aria-modal="true"
      aria-labelledby="choose-mode-title"
    >
      <div
        ref={dialogRef}
        tabIndex={-1}
        className="bg-card border rounded-xl shadow-2xl flex flex-col w-full max-w-md mx-4 outline-none animate-in fade-in zoom-in-95 duration-200"
        onClick={(e) => e.stopPropagation()}
      >
        {/* Header */}
        <div className="flex items-center justify-between px-4 py-3 border-b">
          <div>
            <h2
              id="choose-mode-title"
              className="font-semibold text-base"
            >
              Choose Mode
            </h2>
            <p className="text-xs text-muted-foreground">{subtitle}</p>
          </div>
          {!isAutoConfirm && (
            <Badge variant={canConfirm ? "default" : "secondary"} aria-live="polite">
              {selected.size} / {maxChoices} selected
            </Badge>
          )}
        </div>

        {/* Instructions */}
        <div className="px-4 py-2 bg-blue-50 dark:bg-blue-950/20 border-b">
          <p className="text-sm font-semibold text-blue-700 dark:text-blue-400 text-center">
            {isAutoConfirm
              ? "Click a mode to resolve it."
              : "Select the modes you want to resolve, then confirm."}
          </p>
        </div>

        {/* Mode list */}
        <div className="p-4 flex flex-col gap-2 max-h-[60vh] overflow-y-auto" role="group" aria-label="Available modes">
          {options.map((desc, idx) => {
            const isSelected = selected.has(idx);
            const isDisabled = !isAutoConfirm && !isSelected && selected.size >= maxChoices && maxChoices > 1;
            
            return (
              <button
                key={idx}
                onClick={() => toggleOption(idx)}
                disabled={isDisabled}
                aria-pressed={showCheckboxes ? isSelected : undefined}
                className={cn(
                  "w-full text-left px-4 py-3 rounded-lg border text-sm font-medium transition-all group",
                  "hover:border-primary/50",
                  "disabled:opacity-40 disabled:cursor-not-allowed disabled:hover:border-border",
                  isSelected
                    ? "border-primary bg-primary/10 ring-1 ring-primary"
                    : "border-border bg-background hover:bg-muted/50",
                )}
              >
                <span className="flex items-start gap-3">
                  {showCheckboxes && (
                    <span
                      aria-hidden="true"
                      className={cn(
                        "mt-0.5 inline-flex items-center justify-center w-4 h-4 rounded border shrink-0 transition-colors",
                        isSelected
                          ? "bg-primary border-primary text-primary-foreground"
                          : "border-muted-foreground group-hover:border-primary/50",
                      )}
                    >
                      {isSelected && (
                        <svg className="w-3 h-3" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={3}>
                          <path strokeLinecap="round" strokeLinejoin="round" d="M5 13l4 4L19 7" />
                        </svg>
                      )}
                    </span>
                  )}
                  <span className="leading-tight">{desc}</span>
                </span>
              </button>
            );
          })}
        </div>

        {/* Footer — only shown when we don't auto-confirm */}
        {!isAutoConfirm && (
          <div className="flex justify-between items-center px-4 py-3 border-t bg-muted/10 rounded-b-xl gap-2">
            <span className="text-xs text-muted-foreground text-left leading-tight max-w-[200px]">
              {minChoices === 0 ? "Choosing a mode is optional." : `You must select at least ${minChoices}.`}
            </span>
            <Button
              size="sm"
              disabled={!canConfirm}
              onClick={handleConfirm}
              className="min-w-[100px] shrink-0"
            >
              {minChoices === 0 && selected.size === 0 ? "Skip" : `Confirm ${selected.size > 0 ? `(${selected.size})` : ""}`}
            </Button>
          </div>
        )}
      </div>
    </div>,
    document.body,
  );
}
