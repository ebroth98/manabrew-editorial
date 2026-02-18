import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { User } from "lucide-react";
import type { Table as XMageTable } from "@/types/xmage";

interface TablesListProps {
  tables: XMageTable[];
}

export function TablesList({ tables }: TablesListProps) {
  return (
    <div className="h-full flex flex-col">
      <div className="flex items-center justify-between p-4 border-b">
        <h2 className="text-lg font-semibold">Active Matches</h2>
        <Button size="sm">New Match</Button>
      </div>
      <div className="flex-1 overflow-auto">
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead className="w-[100px]">Format</TableHead>
              <TableHead>Description</TableHead>
              <TableHead>Status</TableHead>
              <TableHead>Players</TableHead>
              <TableHead className="text-right">Action</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {tables.length === 0 ? (
              <TableRow>
                <TableCell colSpan={5} className="text-center py-8 text-muted-foreground">
                  No active matches found.
                </TableCell>
              </TableRow>
            ) : (
              tables.map((table) => (
                <TableRow key={table.id}>
                  <TableCell className="font-medium">{table.gameType}</TableCell>
                  <TableCell>
                    <div className="flex flex-col">
                      <span>{table.name}</span>
                      <span className="text-xs text-muted-foreground">{table.deckType}</span>
                    </div>
                  </TableCell>
                  <TableCell>
                    <Badge variant={table.state === 'WAITING' ? 'outline' : 'secondary'}>
                      {table.state}
                    </Badge>
                  </TableCell>
                  <TableCell>
                    <div className="flex items-center gap-1">
                      <User className="w-3 h-3 text-muted-foreground" />
                      <span>{table.players.length}/{table.numPlayers}</span>
                    </div>
                  </TableCell>
                  <TableCell className="text-right">
                    {table.state === 'WAITING' ? (
                      <Button size="sm" variant="secondary">Join</Button>
                    ) : (
                      <Button size="sm" variant="ghost">Watch</Button>
                    )}
                  </TableCell>
                </TableRow>
              ))
            )}
          </TableBody>
        </Table>
      </div>
    </div>
  );
}
