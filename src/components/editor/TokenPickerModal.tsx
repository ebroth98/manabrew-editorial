import { useMemo, useState } from "react";
import { Modal } from "@/components/game/modals/Modal";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Input } from "@/components/ui/input";
import { peekAllArchivedTokens } from "@/stores/useScryfallStore";
import { ScryfallImg } from "@/components/ScryfallImg";
import type { DeckCard } from "@/types/manabrew";

interface TokenPickerModalProps {
  open: boolean;
  onClose: () => void;
  onSelect: (token: DeckCard) => void;
}

export function TokenPickerModal({ open, onClose, onSelect }: TokenPickerModalProps) {
  const [query, setQuery] = useState("");
  const all = useMemo(() => (open ? peekAllArchivedTokens() : []), [open]);
  const filtered = useMemo(() => {
    if (!query.trim()) return all;
    const q = query.toLowerCase();
    return all.filter(
      (t) =>
        t.name.toLowerCase().includes(q) ||
        t.subtypes.some((s) => s.toLowerCase().includes(q)) ||
        t.types.some((s) => s.toLowerCase().includes(q)),
    );
  }, [all, query]);

  if (!open) return null;

  return (
    <Modal
      onClose={onClose}
      maxWidth="max-w-4xl"
      maxHeight="max-h-[80vh]"
      backdropClassName="z-[9100]"
    >
      <Modal.Header onClose={onClose}>
        <h2 className="text-lg font-bold">Add Token</h2>
      </Modal.Header>

      <div className="px-4 pt-3 pb-2 border-b">
        <Input
          autoFocus
          placeholder="Search token name, type, or subtype…"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
        />
        <div className="text-xs text-muted-foreground mt-1">
          {filtered.length} of {all.length}
        </div>
      </div>

      <Modal.Body>
        <ScrollArea className="h-full">
          <div className="grid grid-cols-3 sm:grid-cols-4 md:grid-cols-5 lg:grid-cols-6 gap-3 p-1">
            {filtered.map((t) => (
              <button
                key={t.id}
                type="button"
                className="group cursor-pointer flex flex-col gap-1 items-center text-left"
                onClick={() => {
                  onSelect(t);
                  onClose();
                }}
              >
                <div className="w-full aspect-[5/7] rounded-[4%] overflow-hidden border-2 border-transparent group-hover:border-primary transition-colors bg-muted">
                  <ScryfallImg
                    src={t.uris.normal}
                    alt={t.name}
                    className="w-full h-full object-cover"
                    loading="lazy"
                    draggable={false}
                  />
                </div>
                <div className="w-full text-center">
                  <div className="text-xs font-semibold truncate" title={t.name}>
                    {t.name}
                  </div>
                  <div className="text-[10px] text-muted-foreground truncate">
                    {[...t.supertypes, ...t.types, ...t.subtypes].join(" ")}
                  </div>
                </div>
              </button>
            ))}
            {filtered.length === 0 && (
              <div className="col-span-full text-center text-sm text-muted-foreground py-8">
                No tokens match your search.
              </div>
            )}
          </div>
        </ScrollArea>
      </Modal.Body>
    </Modal>
  );
}
