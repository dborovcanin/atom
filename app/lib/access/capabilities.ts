export type CapabilityApplicability = {
  objectKind: string;
  objectType?: string | null;
};

export type CapabilityLike = {
  name: string;
  applicability?: CapabilityApplicability[] | null;
};

export type CapabilityTarget = {
  objectKind: string;
  objectType?: string | null;
};

export const CAPABILITY_APPLICABILITY_OPTIONS = [
  { label: "Tenant", objectKind: "tenant", objectType: null },
  { label: "Human users", objectKind: "entity", objectType: "entity:human" },
  {
    label: "Devices / clients",
    objectKind: "entity",
    objectType: "entity:device",
  },
  { label: "Services", objectKind: "entity", objectType: "entity:service" },
  { label: "Workloads", objectKind: "entity", objectType: "entity:workload" },
  {
    label: "Applications",
    objectKind: "entity",
    objectType: "entity:application",
  },
  { label: "Channels", objectKind: "resource", objectType: "resource:channel" },
  { label: "Rules", objectKind: "resource", objectType: "resource:rule" },
  { label: "Reports", objectKind: "resource", objectType: "resource:report" },
  { label: "Alarms", objectKind: "resource", objectType: "resource:alarm" },
  { label: "Groups", objectKind: "group", objectType: null },
  { label: "Roles", objectKind: "role", objectType: null },
  { label: "Assignments / policies", objectKind: "policy", objectType: null },
  { label: "Credentials", objectKind: "credential", objectType: null },
  { label: "Audit logs", objectKind: "audit_log", objectType: null },
  { label: "Signing keys", objectKind: "signing_key", objectType: null },
] as const;

export function encodeApplicability(item: CapabilityApplicability) {
  return `${item.objectKind}|${item.objectType ?? ""}`;
}

export function decodeApplicability(value: string): CapabilityApplicability {
  const [objectKind, objectType = ""] = value.split("|", 2);
  return { objectKind, objectType: objectType || null };
}

export function applicabilityValue(item: CapabilityApplicability) {
  return item.objectType ?? item.objectKind;
}

export function applicabilityValues(capability: CapabilityLike) {
  return (capability.applicability ?? []).map(applicabilityValue);
}

export function applicabilityLabel(capability: CapabilityLike) {
  const values = applicabilityValues(capability);
  return values.length > 0 ? values.join(", ") : "Not assigned to objects";
}

export function capabilityLabel(capability: CapabilityLike) {
  const suffix = applicabilityValues(capability);
  return suffix.length > 0
    ? `${capability.name} (${suffix.join(", ")})`
    : capability.name;
}

export function capabilityAppliesToTarget(
  capability: CapabilityLike,
  target: CapabilityTarget | null,
) {
  if (!target) return true;
  return (capability.applicability ?? []).some(
    (item) =>
      item.objectKind === target.objectKind &&
      (!target.objectType ||
        !item.objectType ||
        item.objectType === target.objectType),
  );
}

export function capabilityTargetFromRoleScope(
  scopeKind: string | null | undefined,
  scopeRef: string | null | undefined,
): CapabilityTarget | null {
  if (!scopeKind || scopeKind === "platform" || scopeKind === "tenant") {
    return null;
  }
  if (scopeKind === "object_kind") {
    return scopeRef ? { objectKind: scopeRef } : null;
  }
  if (scopeKind === "object_type") {
    return targetFromObjectType(scopeRef);
  }
  if (
    scopeKind === "group_object_type" ||
    scopeKind === "group_tree_object_type"
  ) {
    const [, objectKind, ...objectTypeParts] = (scopeRef ?? "").split(":");
    if (!objectKind || objectTypeParts.length === 0) return null;
    return {
      objectKind,
      objectType: `${objectKind}:${objectTypeParts.join(":")}`,
    };
  }
  if (scopeKind === "group_child_kind" || scopeKind === "group_descendant_kind") {
    return { objectKind: "group" };
  }

  // Exact object scopes need object lookup to know their concrete type.
  return null;
}

function targetFromObjectType(value: string | null | undefined) {
  const [objectKind] = (value ?? "").split(":");
  if (!objectKind || !value?.includes(":")) return null;
  return { objectKind, objectType: value };
}
