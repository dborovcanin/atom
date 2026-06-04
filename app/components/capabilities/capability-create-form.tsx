"use client";

import { zodResolver } from "@hookform/resolvers/zod";
import { useMutation, useQuery } from "@tanstack/react-query";
import { useForm } from "react-hook-form";
import { toast } from "sonner";
import { z } from "zod";
import { RequiredFormLabel } from "@/components/forms/required-form-label";
import { Button } from "@/components/ui/button";
import {
  Form,
  FormControl,
  FormDescription,
  FormField,
  FormItem,
  FormLabel,
  FormMessage,
} from "@/components/ui/form";
import { Input } from "@/components/ui/input";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { graphqlClient } from "@/lib/graphql/client";

const CAPABILITIES_QUERY = `
  query ActionApplicabilityFormActions {
    actions(limit: 500, offset: 0) { items { id name description } }
  }
`;

const CREATE_CAPABILITY_MUTATION = `
  mutation CreateAction($input: CreateActionInput!) {
    createAction(input: $input) {
      id
      name
      description
      createdAt
      updatedAt
    }
  }
`;

const ADD_CAPABILITY_APPLICABILITY_MUTATION = `
  mutation AddActionApplicability($input: AddActionApplicabilityInput!) {
    addActionApplicability(input: $input) {
      id
      actionId
      actionName
      objectKind
      objectType
      createdAt
    }
  }
`;

const OBJECT_KINDS = [
  "entity",
  "resource",
  "group",
  "tenant",
  "role",
  "policy",
  "credential",
  "audit_log",
] as const;

export type CapabilityApplicabilityFormInitialValues = {
  id: string;
  capabilityId: string;
  capabilityName: string;
  objectKind: string;
  objectType: string;
};

const actionSchema = z.object({
  name: z.string().trim().min(1, "name is required."),
  description: z.string().trim(),
});

const applicabilitySchema = z.object({
  capabilityId: z.string().min(1, "action_id is required."),
  objectKind: z.string().min(1, "object_kind is required."),
  objectType: z.string().trim(),
});

type CapabilityActionValues = z.infer<typeof actionSchema>;
type CapabilityApplicabilityValues = z.infer<typeof applicabilitySchema>;

type CapabilityOption = {
  id: string;
  name: string;
  description?: string | null;
};

export function CapabilityActionCreateForm({
  onCancel,
  onSaved,
}: {
  onCancel: () => void;
  onSaved: () => void;
}) {
  const form = useForm<CapabilityActionValues>({
    resolver: zodResolver(actionSchema),
    defaultValues: {
      name: "",
      description: "",
    },
  });

  const save = useMutation({
    mutationFn: (values: CapabilityActionValues) =>
      graphqlClient({
        query: CREATE_CAPABILITY_MUTATION,
        variables: {
          input: {
            name: values.name,
            description: values.description || null,
          },
        },
      }),
    onSuccess: () => {
      toast.success("Action created");
      onSaved();
    },
    onError: (err) => toast.error(err.message),
  });

  return (
    <Form {...form}>
      <form
        className="grid gap-4"
        onSubmit={form.handleSubmit((values) => save.mutate(values))}
      >
        <FormField
          control={form.control}
          name="name"
          render={({ field }) => (
            <FormItem>
              <RequiredFormLabel required>name</RequiredFormLabel>
              <FormControl>
                <Input placeholder="e.g. publish" {...field} />
              </FormControl>
              <FormDescription>
                One unique action name stored in `actions.name`.
              </FormDescription>
              <FormMessage />
            </FormItem>
          )}
        />

        <FormField
          control={form.control}
          name="description"
          render={({ field }) => (
            <FormItem>
              <FormLabel>description</FormLabel>
              <FormControl>
                <Input
                  placeholder="e.g. Publish messages to channels"
                  {...field}
                />
              </FormControl>
              <FormMessage />
            </FormItem>
          )}
        />

        <div className="flex justify-end gap-2">
          <Button onClick={onCancel} type="button" variant="outline">
            Cancel
          </Button>
          <Button disabled={save.isPending} type="submit">
            Create action
          </Button>
        </div>
      </form>
    </Form>
  );
}

export function CapabilityApplicabilityCreateForm({
  capability,
  onCancel,
  onSaved,
}: {
  capability?: CapabilityApplicabilityFormInitialValues;
  onCancel: () => void;
  onSaved: () => void;
}) {
  const capabilitiesQuery = useQuery({
    queryKey: ["capability-applicability-form-capabilities"],
    queryFn: ({ signal }) =>
      graphqlClient<{ actions: { items: CapabilityOption[] } }>({
        query: CAPABILITIES_QUERY,
        signal,
      }),
    staleTime: 60_000,
  });

  const form = useForm<CapabilityApplicabilityValues>({
    resolver: zodResolver(applicabilitySchema),
    defaultValues: {
      capabilityId: capability?.capabilityId ?? "",
      objectKind: capability?.objectKind ?? "",
      objectType: capability?.objectType ?? "",
    },
  });

  const save = useMutation({
    mutationFn: (values: CapabilityApplicabilityValues) =>
      graphqlClient({
        query: ADD_CAPABILITY_APPLICABILITY_MUTATION,
        variables: {
          input: {
            actionId: values.capabilityId,
            objectKind: values.objectKind,
            objectType: values.objectType || null,
          },
        },
      }),
    onSuccess: () => {
      toast.success("Action applicability row saved");
      onSaved();
    },
    onError: (err) => toast.error(err.message),
  });

  const capabilities = capabilitiesQuery.data?.actions.items ?? [];

  return (
    <Form {...form}>
      <form
        className="grid gap-4"
        onSubmit={form.handleSubmit((values) => save.mutate(values))}
      >
        <FormField
          control={form.control}
          name="capabilityId"
          render={({ field }) => (
            <FormItem>
              <RequiredFormLabel required>action_id</RequiredFormLabel>
              <Select
                disabled={capabilitiesQuery.isFetching}
                onValueChange={field.onChange}
                value={field.value || undefined}
              >
                <FormControl>
                  <SelectTrigger className="w-full">
                    <SelectValue placeholder="Select action" />
                  </SelectTrigger>
                </FormControl>
                <SelectContent>
                  {capabilities.map((item) => (
                    <SelectItem key={item.id} value={item.id}>
                      {item.name} ({item.id})
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
              <FormDescription>
                References `actions.id`. Create the action first if it does not
                exist yet.
              </FormDescription>
              <FormMessage />
            </FormItem>
          )}
        />

        <FormField
          control={form.control}
          name="objectKind"
          render={({ field }) => (
            <FormItem>
              <RequiredFormLabel required>object_kind</RequiredFormLabel>
              <Select
                onValueChange={field.onChange}
                value={field.value || undefined}
              >
                <FormControl>
                  <SelectTrigger className="w-full">
                    <SelectValue placeholder="Select object_kind" />
                  </SelectTrigger>
                </FormControl>
                <SelectContent>
                  {OBJECT_KINDS.map((kind) => (
                    <SelectItem key={kind} value={kind}>
                      {kind}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
              <FormMessage />
            </FormItem>
          )}
        />

        <FormField
          control={form.control}
          name="objectType"
          render={({ field }) => (
            <FormItem>
              <FormLabel>object_type</FormLabel>
              <FormControl>
                <Input placeholder="NULL or e.g. resource:channel" {...field} />
              </FormControl>
              <FormDescription>
                Leave empty to store `NULL`. Use namespaced values such as
                `entity:device` or `resource:channel`.
              </FormDescription>
              <FormMessage />
            </FormItem>
          )}
        />

        <div className="flex justify-end gap-2">
          <Button onClick={onCancel} type="button" variant="outline">
            Cancel
          </Button>
          <Button disabled={save.isPending} type="submit">
            Create row
          </Button>
        </div>
      </form>
    </Form>
  );
}
