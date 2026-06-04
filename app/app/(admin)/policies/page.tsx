import { AlertTriangle } from "lucide-react";
import type { Metadata } from "next";
import { CrudWorkspace } from "@/components/crud/crud-workspace";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";

export const metadata: Metadata = { title: "Direct Policies" };

export default async function PoliciesPage({
  searchParams,
}: {
  searchParams: Promise<Record<string, string | string[] | undefined>>;
}) {
  const sp = await searchParams;
  return (
    <div className="grid gap-4">
      <Alert>
        <AlertTriangle className="size-4" />
        <AlertTitle>Advanced security surface</AlertTitle>
        <AlertDescription>
          Normal workflows should use roles and role assignments. Direct
          policies are advanced subject-to-permission-block grants for trusted
          service flows and security exceptions.
        </AlertDescription>
      </Alert>
      <CrudWorkspace resourceKey="policies" searchParams={sp} />
    </div>
  );
}
