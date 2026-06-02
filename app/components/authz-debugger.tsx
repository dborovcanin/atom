"use client";

import { zodResolver } from "@hookform/resolvers/zod";
import { useMutation, useQuery } from "@tanstack/react-query";
import {
  AlertTriangle,
  CheckCircle2,
  MinusCircle,
  Play,
  XCircle,
} from "lucide-react";
import * as React from "react";
import { useForm } from "react-hook-form";
import { z } from "zod";
import { RequiredFormLabel } from "@/components/forms/required-form-label";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import {
  Form,
  FormControl,
  FormDescription,
  FormField,
  FormItem,
  FormLabel,
  FormMessage,
} from "@/components/ui/form";
import { JsonEditor } from "@/components/ui/json-editor";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import type { CapabilityApplicability } from "@/lib/access/capabilities";
import { graphqlClient } from "@/lib/graphql/client";

// ─── GraphQL ──────────────────────────────────────────────────────────────────

const ENTITIES_QUERY = `
  query AuthzEntities { entities(limit: 200, offset: 0) { items { id name kind } } }
`;
const TENANTS_QUERY = `
  query AuthzTenants { tenants(limit: 200, offset: 0) { items { id name } } }
`;
const CAPABILITIES_QUERY = `
  query AuthzActions { actions(limit: 200, offset: 0) { items { id name applicability { objectKind objectType } } } }
`;
const RESOURCES_QUERY = `
  query AuthzResources { resources(limit: 200, offset: 0) { items { id name kind } } }
`;
const OBJECT_GROUPS_QUERY = `
  query AuthzObjectGroups { objectGroups(limit: 200, offset: 0) { items { id name tenantId } } }
`;
const EXPLAIN_MUTATION = `
  mutation Explain($input: AuthzCheckInput!) {
    authzExplain(input: $input) {
      allowed
      reason
      matchedBinding
      evaluatedBindings
    }
  }
`;

// ─── Schema ───────────────────────────────────────────────────────────────────

const NONE = "__none__";

const TARGET_KINDS = [
  {
    value: "platform",
    label: "Platform",
    description: "Global platform-level check. No target object is needed.",
  },
  {
    value: "tenant",
    label: "Tenant",
    description: "The tenant/domain object itself.",
  },
  {
    value: "entity",
    label: "Entity",
    description: "A human user, client/device, service, workload, or app.",
  },
  {
    value: "resource",
    label: "Resource",
    description: "A channel, rule, report, alarm, or other resource.",
  },
  {
    value: "group",
    label: "Object group",
    description: "The group object itself, not the objects inside it.",
  },
] as const;

type TargetKind = (typeof TARGET_KINDS)[number]["value"];

const schema = z
  .object({
    subjectId: z.string().min(1, "Subject is required"),
    action: z.string().min(1, "Action is required"),
    targetKind: z.enum(["platform", "tenant", "entity", "resource", "group"]),
    targetId: z.string().optional(),
    context: z.string(),
  })
  .superRefine((values, ctx) => {
    if (values.targetKind !== "platform" && !values.targetId) {
      ctx.addIssue({
        code: z.ZodIssueCode.custom,
        path: ["targetId"],
        message: "Target is required",
      });
    }
  });
type FormValues = z.infer<typeof schema>;

type ExplainResponse = {
  authzExplain: {
    allowed: boolean;
    reason: string;
    matchedBinding?: Record<string, unknown> | null;
    evaluatedBindings: Array<Record<string, unknown>>;
  };
};

// ─── Component ────────────────────────────────────────────────────────────────

export function AuthzDebugger() {
  const entitiesQ = useQuery({
    queryKey: ["authz-entities"],
    queryFn: ({ signal }) =>
      graphqlClient<{
        entities: { items: EntityOption[] };
      }>({
        query: ENTITIES_QUERY,
        signal,
      }),
    staleTime: 60_000,
  });
  const tenantsQ = useQuery({
    queryKey: ["authz-tenants"],
    queryFn: ({ signal }) =>
      graphqlClient<{ tenants: { items: TenantOption[] } }>({
        query: TENANTS_QUERY,
        signal,
      }),
    staleTime: 60_000,
  });
  const actionsQ = useQuery({
    queryKey: ["authz-actions"],
    queryFn: ({ signal }) =>
      graphqlClient<{
        actions: {
          items: ActionItem[];
        };
      }>({
        query: CAPABILITIES_QUERY,
        signal,
      }),
    staleTime: 60_000,
  });
  const resourcesQ = useQuery({
    queryKey: ["authz-resources"],
    queryFn: ({ signal }) =>
      graphqlClient<{
        resources: { items: ResourceOption[] };
      }>({
        query: RESOURCES_QUERY,
        signal,
      }),
    staleTime: 60_000,
  });
  const objectGroupsQ = useQuery({
    queryKey: ["authz-object-groups"],
    queryFn: ({ signal }) =>
      graphqlClient<{ objectGroups: { items: GroupOption[] } }>({
        query: OBJECT_GROUPS_QUERY,
        signal,
      }),
    staleTime: 60_000,
  });

  const entities = entitiesQ.data?.entities.items ?? [];
  const tenants = tenantsQ.data?.tenants.items ?? [];
  const actions = actionsQ.data?.actions.items ?? [];
  const resources = resourcesQ.data?.resources.items ?? [];
  const objectGroups = objectGroupsQ.data?.objectGroups.items ?? [];

  const form = useForm<FormValues>({
    resolver: zodResolver(schema),
    defaultValues: {
      subjectId: "",
      action: "",
      targetKind: "resource",
      targetId: "",
      context: "{}",
    },
  });

  const targetKind = form.watch("targetKind");
  const targetOptions = React.useMemo(
    () =>
      targetOptionsFor(targetKind, {
        tenants,
        entities,
        resources,
        objectGroups,
      }),
    [targetKind, tenants, entities, resources, objectGroups],
  );
  const targetKindMeta = TARGET_KINDS.find((item) => item.value === targetKind);
  const selectedSubject = entities.find(
    (entity) => entity.id === form.watch("subjectId"),
  );
  const selectedAction = actions.find(
    (action) => action.id === form.watch("action"),
  );
  const selectedTarget = targetOptions.find(
    (target) => target.id === form.watch("targetId"),
  );

  const explain = useMutation({
    mutationFn: (values: FormValues) => {
      const action = actions.find((item) => item.id === values.action);
      const context = parseContext(values.context);
      return graphqlClient<ExplainResponse>({
        query: EXPLAIN_MUTATION,
        variables: {
          input: buildAuthzInput({
            subjectId: values.subjectId,
            action: action?.name ?? values.action,
            targetKind: values.targetKind,
            targetId: values.targetId,
            context,
          }),
        },
      });
    },
  });

  const result = explain.data?.authzExplain;

  return (
    <div className="grid gap-4 xl:grid-cols-[460px_1fr]">
      {/* ── Input form ── */}
      <Card>
        <CardHeader>
          <CardTitle>Authorization request</CardTitle>
          <CardDescription>
            Read this as: who wants to do what on which target.
          </CardDescription>
        </CardHeader>
        <CardContent>
          <Form {...form}>
            <form
              className="grid gap-4"
              onSubmit={form.handleSubmit((v) => explain.mutate(v))}
            >
              <FormField
                control={form.control}
                name="subjectId"
                render={({ field }) => (
                  <FormItem>
                    <RequiredFormLabel required>Who</RequiredFormLabel>
                    <Select
                      value={field.value || NONE}
                      onValueChange={(v) => field.onChange(v === NONE ? "" : v)}
                    >
                      <FormControl>
                        <SelectTrigger className="w-full">
                          <SelectValue />
                        </SelectTrigger>
                      </FormControl>
                      <SelectContent>
                        <SelectItem value={NONE}>Select subject</SelectItem>
                        {entities.map((e) => (
                          <SelectItem key={e.id} value={e.id}>
                            {e.name}
                            <span className="ml-1.5 text-xs text-muted-foreground">
                              {e.kind}
                            </span>
                          </SelectItem>
                        ))}
                      </SelectContent>
                    </Select>
                    <FormDescription>
                      The entity making the request, for example a user, client,
                      or service.
                    </FormDescription>
                    <FormMessage />
                  </FormItem>
                )}
              />

              <FormField
                control={form.control}
                name="action"
                render={({ field }) => (
                  <FormItem>
                    <RequiredFormLabel required>Can do</RequiredFormLabel>
                    <Select
                      value={field.value || NONE}
                      onValueChange={(v) => field.onChange(v === NONE ? "" : v)}
                    >
                      <FormControl>
                        <SelectTrigger className="w-full">
                          <SelectValue />
                        </SelectTrigger>
                      </FormControl>
                      <SelectContent>
                        <SelectItem value={NONE}>Select action</SelectItem>
                        {actions.map((action) => (
                          <SelectItem key={action.id} value={action.id}>
                            {action.name}
                          </SelectItem>
                        ))}
                      </SelectContent>
                    </Select>
                    <FormDescription>
                      Action names are the canonical Atom permissions, such as
                      read, write, publish, role.manage, or policy.manage.
                    </FormDescription>
                    <FormMessage />
                  </FormItem>
                )}
              />

              <FormField
                control={form.control}
                name="targetKind"
                render={({ field }) => (
                  <FormItem>
                    <RequiredFormLabel required>Target type</RequiredFormLabel>
                    <Select
                      value={field.value}
                      onValueChange={(v) => {
                        const next = v as TargetKind;
                        field.onChange(next);
                        form.setValue("targetId", "");
                      }}
                    >
                      <FormControl>
                        <SelectTrigger className="w-full">
                          <SelectValue />
                        </SelectTrigger>
                      </FormControl>
                      <SelectContent>
                        {TARGET_KINDS.map((kind) => (
                          <SelectItem key={kind.value} value={kind.value}>
                            {kind.label}
                          </SelectItem>
                        ))}
                      </SelectContent>
                    </Select>
                    <FormDescription>
                      {targetKindMeta?.description}
                    </FormDescription>
                    <FormMessage />
                  </FormItem>
                )}
              />

              {targetKind !== "platform" && (
                <FormField
                  control={form.control}
                  name="targetId"
                  render={({ field }) => (
                    <FormItem>
                      <RequiredFormLabel required>
                        {targetKindMeta?.label ?? "Target"}
                      </RequiredFormLabel>
                      <Select
                        value={field.value || NONE}
                        onValueChange={(v) =>
                          field.onChange(v === NONE ? "" : v)
                        }
                      >
                        <FormControl>
                          <SelectTrigger className="w-full">
                            <SelectValue />
                          </SelectTrigger>
                        </FormControl>
                        <SelectContent>
                          <SelectItem value={NONE}>Select target</SelectItem>
                          {targetOptions.map((target) => (
                            <SelectItem key={target.id} value={target.id}>
                              {target.label}
                              {target.detail ? (
                                <span className="ml-1.5 text-xs text-muted-foreground">
                                  {target.detail}
                                </span>
                              ) : null}
                            </SelectItem>
                          ))}
                        </SelectContent>
                      </Select>
                      {targetOptions.length === 0 ? (
                        <FormDescription>
                          No targets of this type were returned for the current
                          user.
                        </FormDescription>
                      ) : null}
                      <FormMessage />
                    </FormItem>
                  )}
                />
              )}

              <div className="rounded-lg border bg-muted/30 p-3 text-sm">
                <div className="mb-1 text-xs font-medium uppercase text-muted-foreground">
                  Request preview
                </div>
                <div>
                  <span className="font-medium">
                    {selectedSubject?.name ?? "Selected subject"}
                  </span>{" "}
                  wants to{" "}
                  <span className="font-medium">
                    {selectedAction?.name ?? "selected action"}
                  </span>{" "}
                  on{" "}
                  <span className="font-medium">
                    {targetKind === "platform"
                      ? "the platform"
                      : (selectedTarget?.label ?? "selected target")}
                  </span>
                  .
                </div>
              </div>

              <FormField
                control={form.control}
                name="context"
                render={({ field }) => (
                  <FormItem>
                    <FormLabel>Optional context JSON</FormLabel>
                    <FormControl>
                      <JsonEditor
                        value={field.value}
                        onChange={field.onChange}
                        className="[&_.cm-editor]:min-h-16"
                      />
                    </FormControl>
                    <FormDescription>
                      Extra request attributes used by conditional permissions.
                      Keep <code>{"{}"}</code> for a normal check.
                    </FormDescription>
                    <FormMessage />
                  </FormItem>
                )}
              />

              <Button
                type="submit"
                disabled={explain.isPending}
                className="w-full"
              >
                <Play className="size-4" />
                Explain decision
              </Button>
            </form>
          </Form>
        </CardContent>
      </Card>

      {/* ── Results ── */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            {result?.allowed ? (
              <CheckCircle2 className="text-primary" />
            ) : result ? (
              <XCircle className="text-destructive" />
            ) : (
              <AlertTriangle className="text-muted-foreground" />
            )}
            Decision
          </CardTitle>
          <CardDescription>
            Atom checks direct policies and role assignments, then applies
            deny-overrides-allow.
          </CardDescription>
        </CardHeader>
        <CardContent className="grid gap-4">
          {explain.isError ? (
            <Alert variant="destructive">
              <AlertTriangle className="size-4" />
              <AlertTitle>Explain failed</AlertTitle>
              <AlertDescription>{explain.error.message}</AlertDescription>
            </Alert>
          ) : null}

          {result ? (
            <>
              <div className="rounded-lg border p-4">
                <Badge variant={result.allowed ? "default" : "destructive"}>
                  {result.allowed ? "Allowed" : "Denied"}
                </Badge>
                <p className="mt-3 text-lg">{result.reason}</p>
              </div>
              <TraceList
                title="Matched permission"
                items={result.matchedBinding ? [result.matchedBinding] : []}
                actions={actions}
              />
              <TraceList
                title="Permissions checked"
                items={result.evaluatedBindings ?? []}
                actions={actions}
              />
            </>
          ) : (
            <div className="rounded-lg border border-dashed p-8 text-center text-sm text-muted-foreground">
              Choose who, action, and target to see the decision.
            </div>
          )}
        </CardContent>
      </Card>
    </div>
  );
}

// ─── Trace display ────────────────────────────────────────────────────────────

type ActionItem = {
  id: string;
  name: string;
  applicability: CapabilityApplicability[];
};

type EntityOption = { id: string; name: string; kind: string };
type TenantOption = { id: string; name: string };
type ResourceOption = { id: string; name: string; kind: string };
type GroupOption = { id: string; name: string; tenantId?: string | null };
type TargetOption = { id: string; label: string; detail?: string };

function targetOptionsFor(
  targetKind: TargetKind,
  data: {
    tenants: TenantOption[];
    entities: EntityOption[];
    resources: ResourceOption[];
    objectGroups: GroupOption[];
  },
): TargetOption[] {
  switch (targetKind) {
    case "platform":
      return [];
    case "tenant":
      return data.tenants.map((tenant) => ({
        id: tenant.id,
        label: tenant.name,
      }));
    case "entity":
      return data.entities.map((entity) => ({
        id: entity.id,
        label: entity.name,
        detail: entity.kind,
      }));
    case "resource":
      return data.resources.map((resource) => ({
        id: resource.id,
        label: resource.name,
        detail: resource.kind,
      }));
    case "group":
      return data.objectGroups.map((group) => ({
        id: group.id,
        label: group.name,
        detail: group.tenantId ? `tenant ${group.tenantId.slice(0, 8)}...` : "",
      }));
  }
}

function parseContext(value: string): Record<string, unknown> {
  try {
    const parsed = JSON.parse(value || "{}") as unknown;
    if (!parsed || Array.isArray(parsed) || typeof parsed !== "object") {
      throw new Error("Context must be a JSON object");
    }
    return parsed as Record<string, unknown>;
  } catch (error) {
    if (
      error instanceof Error &&
      error.message === "Context must be a JSON object"
    ) {
      throw error;
    }
    throw new Error("Context must be valid JSON");
  }
}

function buildAuthzInput(values: {
  subjectId: string;
  action: string;
  targetKind: TargetKind;
  targetId?: string;
  context: Record<string, unknown>;
}) {
  const input: Record<string, unknown> = {
    subjectId: values.subjectId,
    action: values.action,
    context: values.context,
  };

  if (values.targetKind === "platform") {
    input.objectKind = "platform";
    return input;
  }

  input.objectKind = values.targetKind;
  input.objectId = values.targetId;
  return input;
}

function authzScopeSummary(kind: string, ref?: string) {
  switch (kind) {
    case "platform":
      return "entire platform";
    case "tenant":
      return ref ? `tenant ${ref}` : "tenant";
    case "object_kind":
      return ref ? `all ${ref} objects` : "all objects of a kind";
    case "object_type":
      return ref ? `all ${ref}` : "all objects of a type";
    case "object":
      return ref ? `exact object ${ref}` : "exact object";
    case "group":
      return ref ? `object group ${ref}` : "object group";
    case "group_direct_objects":
    case "group_object_type":
      return ref ? `direct objects in group ${ref}` : "direct group objects";
    case "group_descendant_objects":
    case "group_tree_object_type":
      return ref ? `objects in subgroups of ${ref}` : "objects in subgroups";
    case "group_child_groups":
    case "group_child_kind":
      return ref ? `direct child groups of ${ref}` : "direct child groups";
    case "group_descendant_groups":
    case "group_descendant_kind":
      return ref ? `descendant groups of ${ref}` : "descendant groups";
    default:
      return ref ? `${kind} ${ref}` : kind;
  }
}

const SKIP_REASON_LABELS: Record<string, string> = {
  scope_mismatch: "Permission scope does not cover this target",
  grant_mismatch: "Action not covered by this grant",
  conditions_mismatch: "ABAC conditions not satisfied",
};

function TraceList({
  title,
  items,
  actions,
}: {
  title: string;
  items: Array<Record<string, unknown>>;
  actions: ActionItem[];
}) {
  return (
    <div className="grid gap-2">
      <div className="text-sm font-medium">{title}</div>
      {items.length ? (
        items.map((item) => (
          <TraceCard
            key={`${title}-${String(item.id ?? item.scope_ref ?? JSON.stringify(item))}`}
            item={item}
            actions={actions}
          />
        ))
      ) : (
        <div className="rounded-lg border border-dashed p-3 text-sm text-muted-foreground">
          No permissions returned.
        </div>
      )}
    </div>
  );
}

function TraceCard({
  item,
  actions,
}: {
  item: Record<string, unknown>;
  actions: ActionItem[];
}) {
  const effect = String(item.effect ?? "allow");
  const result = String(item.result ?? "skipped");
  const via = String(item.via ?? "direct");
  const grantKind = String(item.grant_kind ?? "capability");
  const grantId = item.grant_id ? String(item.grant_id) : null;
  const roleName = item.role_name ? String(item.role_name) : null;
  const scopeKind = String(item.scope_kind ?? "platform");
  const scopeRef = item.scope_ref ? String(item.scope_ref) : undefined;
  const skipReason = item.skip_reason ? String(item.skip_reason) : null;

  const capName = grantId
    ? (actions.find((c) => c.id === grantId)?.name ?? null)
    : null;
  const grantLabel =
    grantKind === "role"
      ? (roleName ?? (grantId ? `${grantId.slice(0, 8)}…` : "unknown role"))
      : (capName ?? (grantId ? `${grantId.slice(0, 8)}…` : "unknown action"));

  const viaLabel = via.startsWith("group:")
    ? `Inherited through principal group "${via.slice(6)}"`
    : "Direct assignment or policy";

  const isMatched = result === "matched";
  const isDeny = effect === "deny";

  const accentClass = isMatched
    ? isDeny
      ? "border-l-destructive"
      : "border-l-primary"
    : "border-l-muted-foreground/30";

  const headline = isMatched
    ? isDeny
      ? "This permission denies the request"
      : "This permission allows the request"
    : `Skipped — ${SKIP_REASON_LABELS[skipReason ?? ""] ?? skipReason ?? "not applicable"}`;

  const HeadlineIcon = isMatched
    ? isDeny
      ? XCircle
      : CheckCircle2
    : MinusCircle;

  const iconClass = isMatched
    ? isDeny
      ? "text-destructive"
      : "text-primary"
    : "text-muted-foreground";

  const rows: Array<{ label: string; value: string; mono?: boolean }> = [
    {
      label: "Effect",
      value: effect === "allow" ? "Allow" : "Deny",
    },
    {
      label: grantKind === "role" ? "Role" : "Action",
      value: grantLabel,
    },
    {
      label: "Applies to",
      value: authzScopeSummary(scopeKind, scopeRef),
    },
    {
      label: "Via",
      value: viaLabel,
    },
  ];

  return (
    <div
      className={`grid gap-3 rounded-lg border border-l-4 p-3 text-sm ${accentClass}`}
    >
      <div className={`flex items-center gap-2 font-medium ${iconClass}`}>
        <HeadlineIcon className="size-4 shrink-0" />
        {headline}
      </div>
      <div className="grid gap-1.5">
        {rows.map(({ label, value, mono }) => (
          <div key={label} className="flex gap-3">
            <span className="w-24 shrink-0 text-xs font-medium uppercase tracking-wide text-muted-foreground">
              {label}
            </span>
            <span className={`text-xs ${mono ? "font-mono" : ""}`}>
              {value}
            </span>
          </div>
        ))}
      </div>
    </div>
  );
}
