use axum::response::Html;

pub async fn graphql_console() -> Html<&'static str> {
    Html(console_html())
}

pub(crate) fn console_html() -> &'static str {
    r###"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>Atom GraphQL Console</title>
  <style>
    :root {
      color-scheme: light;
      --bg: #f4f6f8;
      --panel: #ffffff;
      --panel-soft: #f8fafc;
      --panel-tint: #eef7f6;
      --border: #d6dee8;
      --border-strong: #b9c5d3;
      --text: #17202a;
      --muted: #5d6d80;
      --soft: #edf2f7;
      --accent: #0f766e;
      --accent-dark: #115e59;
      --accent-soft: #dff5f2;
      --danger: #b42318;
      --warn: #9a6700;
      --code: #101828;
      --code-bg: #f8fafc;
      --shadow: 0 1px 2px rgba(16, 24, 40, .06);
    }

    * { box-sizing: border-box; }

    body {
      margin: 0;
      background: var(--bg);
      color: var(--text);
      font-family: Inter, ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
      min-height: 100vh;
    }

    button, input, select, textarea {
      font: inherit;
    }

    button {
      border: 1px solid var(--border);
      background: var(--panel);
      color: var(--text);
      border-radius: 6px;
      padding: 8px 10px;
      cursor: pointer;
      min-height: 36px;
    }

    button:hover {
      border-color: var(--border-strong);
    }

    button.primary {
      background: var(--accent);
      border-color: var(--accent);
      color: #fff;
      font-weight: 650;
    }

    button.primary:hover {
      background: var(--accent-dark);
      border-color: var(--accent-dark);
    }

    button.ghost {
      background: transparent;
    }

    input, select, textarea {
      width: 100%;
      border: 1px solid var(--border);
      border-radius: 6px;
      background: #fff;
      color: var(--text);
      padding: 8px 9px;
      min-height: 36px;
    }

    textarea {
      min-height: 140px;
      resize: vertical;
      font-family: "SFMono-Regular", Consolas, "Liberation Mono", monospace;
      font-size: 13px;
      line-height: 1.45;
      tab-size: 2;
    }

    label {
      display: grid;
      gap: 5px;
      color: var(--muted);
      font-size: 12px;
    }

    h1, h2, h3, p {
      margin-top: 0;
    }

    h1 {
      font-size: 19px;
      line-height: 1.25;
      margin-bottom: 4px;
      letter-spacing: 0;
    }

    h2 {
      font-size: 16px;
      line-height: 1.3;
      margin-bottom: 10px;
      letter-spacing: 0;
    }

    h3 {
      font-size: 13px;
      color: var(--muted);
      text-transform: uppercase;
      margin: 16px 0 8px;
      letter-spacing: 0;
    }

    p {
      line-height: 1.5;
    }

    .app-shell {
      display: grid;
      grid-template-columns: minmax(250px, 300px) minmax(0, 1fr) minmax(250px, 300px);
      min-height: 100vh;
    }

    .side-nav, .docs-panel {
      background: var(--panel);
      border-right: 1px solid var(--border);
      min-width: 0;
    }

    .docs-panel {
      border-right: 0;
      border-left: 1px solid var(--border);
    }

    .side-scroll, .docs-scroll, .workspace {
      height: 100vh;
      overflow: auto;
    }

    .brand {
      padding: 18px 18px 14px;
      border-bottom: 1px solid var(--border);
    }

    .brand .subtitle {
      color: var(--muted);
      font-size: 13px;
    }

    .nav-section {
      padding: 14px 16px;
      border-bottom: 1px solid var(--border);
    }

    .nav-list {
      display: grid;
      gap: 6px;
    }

    .nav-button {
      width: 100%;
      display: grid;
      gap: 2px;
      text-align: left;
      padding: 9px 10px;
      border-color: transparent;
      background: transparent;
    }

    .nav-button.active {
      background: var(--accent-soft);
      border-color: #a9ded8;
      color: var(--accent-dark);
    }

    .nav-button strong {
      font-size: 13px;
    }

    .nav-button span {
      color: var(--muted);
      font-size: 12px;
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
    }

    .main {
      min-width: 0;
      display: grid;
      grid-template-rows: auto minmax(0, 1fr);
      height: 100vh;
    }

    .topbar {
      background: var(--panel);
      border-bottom: 1px solid var(--border);
      padding: 12px;
      display: grid;
      grid-template-columns: minmax(130px, 210px) minmax(180px, 1fr) minmax(150px, 220px) auto auto;
      gap: 9px;
      align-items: end;
    }

    .workspace {
      padding: 14px;
      min-width: 0;
    }

    .screen {
      display: none;
      max-width: 1180px;
      margin: 0 auto;
    }

    .screen.active {
      display: block;
    }

    .panel {
      background: var(--panel);
      border: 1px solid var(--border);
      border-radius: 8px;
      box-shadow: var(--shadow);
      padding: 14px;
      margin-bottom: 14px;
    }

    .panel.tint {
      background: var(--panel-tint);
    }

    .grid {
      display: grid;
      grid-template-columns: repeat(2, minmax(0, 1fr));
      gap: 10px;
    }

    .grid-3 {
      display: grid;
      grid-template-columns: repeat(3, minmax(0, 1fr));
      gap: 10px;
    }

    .split {
      display: grid;
      grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
      gap: 14px;
    }

    .actions {
      display: flex;
      gap: 8px;
      flex-wrap: wrap;
      align-items: center;
      margin-top: 10px;
    }

    .muted {
      color: var(--muted);
      font-size: 13px;
    }

    .help {
      color: var(--muted);
      font-size: 13px;
      line-height: 1.45;
    }

    .status-row {
      display: flex;
      gap: 8px;
      flex-wrap: wrap;
      align-items: center;
    }

    .badge {
      display: inline-flex;
      align-items: center;
      gap: 5px;
      border: 1px solid var(--border);
      border-radius: 999px;
      padding: 3px 8px;
      background: var(--panel-soft);
      color: var(--muted);
      font-size: 12px;
      max-width: 100%;
    }

    .badge.ok {
      color: var(--accent-dark);
      background: var(--accent-soft);
      border-color: #b9e4df;
    }

    .badge.warn {
      color: var(--warn);
      background: #fff7df;
      border-color: #ead49a;
    }

    .badge.error {
      color: var(--danger);
      background: #fff0ed;
      border-color: #f0b8ae;
    }

    .checklist {
      display: grid;
      grid-template-columns: repeat(4, minmax(0, 1fr));
      gap: 10px;
      margin-top: 12px;
    }

    .step-card {
      border: 1px solid var(--border);
      background: var(--panel);
      border-radius: 8px;
      padding: 12px;
      min-height: 118px;
    }

    .step-card.done {
      border-color: #a9ded8;
      background: #f0fbf9;
    }

    .step-card strong {
      display: block;
      margin-bottom: 5px;
      font-size: 14px;
    }

    .task-grid {
      display: grid;
      grid-template-columns: repeat(3, minmax(0, 1fr));
      gap: 10px;
    }

    .task-button {
      text-align: left;
      min-height: 92px;
      padding: 12px;
    }

    .task-button strong {
      display: block;
      margin-bottom: 6px;
    }

    .operation-list {
      display: grid;
      gap: 7px;
      max-height: 300px;
      overflow: auto;
      padding-right: 2px;
    }

    .operation-button {
      text-align: left;
      display: grid;
      gap: 4px;
      padding: 9px;
    }

    .operation-button strong {
      font-size: 13px;
    }

    .operation-button span {
      color: var(--muted);
      font-size: 12px;
      white-space: nowrap;
      overflow: hidden;
      text-overflow: ellipsis;
    }

    .type-list {
      max-height: 240px;
      overflow: auto;
      display: flex;
      gap: 5px;
      flex-wrap: wrap;
    }

    .pill {
      display: inline-flex;
      align-items: center;
      border: 1px solid var(--border);
      border-radius: 999px;
      padding: 3px 8px;
      font-size: 12px;
      color: var(--muted);
      background: var(--code-bg);
      margin: 1px;
    }

    .field-list {
      display: grid;
      grid-template-columns: repeat(auto-fit, minmax(150px, 1fr));
      gap: 6px 10px;
      max-height: 180px;
      overflow: auto;
      padding: 9px;
      border: 1px solid var(--border);
      border-radius: 6px;
      background: var(--code-bg);
    }

    .field-list label {
      display: flex;
      gap: 6px;
      align-items: center;
      color: var(--text);
      font-size: 13px;
    }

    pre {
      margin: 0;
      white-space: pre-wrap;
      word-break: break-word;
      background: var(--code-bg);
      color: var(--code);
      border: 1px solid var(--border);
      border-radius: 6px;
      padding: 10px;
      min-height: 150px;
      max-height: 390px;
      overflow: auto;
      font-family: "SFMono-Regular", Consolas, "Liberation Mono", monospace;
      font-size: 13px;
      line-height: 1.45;
    }

    .response-header {
      display: flex;
      align-items: center;
      justify-content: space-between;
      gap: 10px;
      margin-bottom: 8px;
    }

    .schema-docs {
      display: grid;
      gap: 10px;
      font-size: 13px;
      line-height: 1.45;
    }

    .schema-docs dt {
      font-weight: 700;
      color: var(--text);
    }

    .schema-docs dd {
      margin: 2px 0 8px;
      color: var(--muted);
    }

    .notice {
      border: 1px solid #e9d7a6;
      background: #fff9e9;
      color: #6b4d00;
      border-radius: 8px;
      padding: 10px;
      font-size: 13px;
      line-height: 1.45;
    }

    .hidden {
      display: none;
    }

    @media (max-width: 1160px) {
      .app-shell {
        grid-template-columns: 260px minmax(0, 1fr);
      }

      .docs-panel {
        display: none;
      }

      .topbar {
        grid-template-columns: minmax(140px, 1fr) minmax(160px, 1fr) minmax(140px, 1fr);
      }
    }

    @media (max-width: 820px) {
      .app-shell {
        display: block;
      }

      .side-scroll, .docs-scroll, .workspace, .main {
        height: auto;
      }

      .side-nav {
        border-right: 0;
        border-bottom: 1px solid var(--border);
      }

      .main {
        display: block;
      }

      .topbar, .grid, .grid-3, .split, .checklist, .task-grid {
        grid-template-columns: 1fr;
      }
    }
  </style>
</head>
<body>
  <div class="app-shell">
    <aside class="side-nav">
      <div class="side-scroll">
        <div class="brand">
          <h1>Atom GraphQL Console</h1>
          <div class="subtitle">Guided tools plus a raw GraphQL explorer.</div>
        </div>

        <section class="nav-section">
          <h2>Start</h2>
          <div class="nav-list">
            <button class="nav-button active" data-screen="start"><strong>Getting Started</strong><span>Connect, login, choose a task</span></button>
            <button class="nav-button" data-screen="login"><strong>Login helper</strong><span>Get a token for requests</span></button>
          </div>
        </section>

        <section class="nav-section">
          <h2>Build</h2>
          <div class="nav-list">
            <button class="nav-button" data-screen="tenant"><strong>Tenant builder</strong><span>Create an isolation boundary</span></button>
            <button class="nav-button" data-screen="profile"><strong>Profile builder</strong><span>Define subtype and schema</span></button>
            <button class="nav-button" data-screen="entity"><strong>Entity builder</strong><span>Create principals from profiles</span></button>
            <button class="nav-button" data-screen="resource"><strong>Resource builder</strong><span>Create protected objects</span></button>
            <button class="nav-button" data-screen="policy"><strong>Policy builder</strong><span>Grant capability or role access</span></button>
            <button class="nav-button" data-screen="authz"><strong>Authz builder</strong><span>Run check or explain</span></button>
          </div>
        </section>

        <section class="nav-section">
          <h2>Explore</h2>
          <label>Search schema
            <input id="schemaSearch" placeholder="filter operations and types" autocomplete="off" />
          </label>
          <div class="actions">
            <button id="refreshSchema">Refresh schema</button>
          </div>
          <div id="schemaStatus" class="status-row" style="margin-top: 10px;">
            <span class="badge warn">Loading schema</span>
          </div>
          <h3>Queries</h3>
          <div id="queryOps" class="operation-list"></div>
          <h3>Mutations</h3>
          <div id="mutationOps" class="operation-list"></div>
          <div class="actions">
            <button class="nav-button" data-screen="explorer"><strong>Open operation explorer</strong><span>Edit and run generated GraphQL</span></button>
          </div>
        </section>
      </div>
    </aside>

    <main class="main">
      <div class="topbar">
        <label>Endpoint
          <input id="endpoint" value="/graphql" autocomplete="off" />
        </label>
        <label>Authorization Bearer token
          <input id="token" type="password" autocomplete="off" placeholder="Use login helper or paste token" />
        </label>
        <label>Saved examples
          <select id="savedExamples"></select>
        </label>
        <button id="loadExample">Load</button>
        <button id="saveExample">Save</button>
        <div id="connectionStatus" class="status-row" style="grid-column: 1 / -1;">
          <span class="badge warn">Schema not loaded</span>
          <span class="badge warn">Not authenticated</span>
        </div>
      </div>

      <div class="workspace">
        <section id="screen-start" class="screen active">
          <div class="panel tint">
            <h2>Getting Started</h2>
            <p class="help">Use this console to inspect Atom's generic GraphQL schema, build operations, and run them against <code>/graphql</code>. The console never inspects raw database tables and never sends your token outside this endpoint.</p>
            <div class="checklist">
              <div id="stepEndpoint" class="step-card">
                <strong>1. Connect</strong>
                <div class="help">Endpoint is fixed to <code>/graphql</code> for token safety.</div>
              </div>
              <div id="stepSchema" class="step-card">
                <strong>2. Load schema</strong>
                <div class="help">GraphQL introspection fills queries, mutations, types, inputs, and enums.</div>
              </div>
              <div id="stepLogin" class="step-card">
                <strong>3. Login</strong>
                <div class="help">Use the login helper or paste a Bearer token.</div>
              </div>
              <div class="step-card">
                <strong>4. Choose a task</strong>
                <div class="help">Create tenants, entities, resources, policies, or run authz checks.</div>
              </div>
            </div>
          </div>

          <div class="panel">
            <h2>Common tasks</h2>
            <div class="task-grid">
              <button class="task-button" data-screen="login"><strong>Login</strong><span class="help">Get an Atom JWT using a password credential.</span></button>
              <button class="task-button" data-screen="tenant"><strong>Create tenant</strong><span class="help">Create an isolation boundary for objects.</span></button>
              <button class="task-button" data-screen="profile"><strong>Create profile</strong><span class="help">Define an entity subtype and attribute schema.</span></button>
              <button class="task-button" data-screen="entity"><strong>Create entity</strong><span class="help">Pick a profile; Atom derives internal kind.</span></button>
              <button class="task-button" data-screen="resource"><strong>Create resource</strong><span class="help">Create a protected object such as channel/rule/report.</span></button>
              <button class="task-button" data-screen="policy"><strong>Create policy</strong><span class="help">Grant capability or role access over a scope.</span></button>
              <button class="task-button" data-screen="authz"><strong>Run authz</strong><span class="help">Check or explain an authorization decision.</span></button>
            </div>
          </div>
        </section>

        <section id="screen-login" class="screen">
          <div class="panel">
            <h2>Login helper</h2>
            <p class="help">Login is public. The returned token is stored only in your browser localStorage and used as a Bearer token for future console requests.</p>
            <div class="grid-3">
              <label>Identifier<input id="loginIdentifier" value="atom-admin" autocomplete="username" /></label>
              <label>Secret<input id="loginSecret" type="password" value="change-me" autocomplete="current-password" /></label>
              <label>Credential kind<input id="loginKind" value="password" /></label>
            </div>
            <div class="actions">
              <button class="primary" id="runLogin">Run login</button>
              <button id="clearToken">Clear token</button>
            </div>
            <pre id="loginResult"></pre>
          </div>
        </section>

        <section id="screen-explorer" class="screen">
          <div class="panel">
            <h2>Operation explorer</h2>
            <div id="selectedOperation" class="help">Select a query or mutation from the left panel. Required arguments appear in the generated variables skeleton.</div>
            <div id="returnFields" class="field-list hidden"></div>
            <div class="actions">
              <button id="copyQuery">Copy query</button>
              <button class="primary" id="runOperation">Run operation</button>
            </div>
          </div>
          <div class="split">
            <div class="panel">
              <h2>GraphQL</h2>
              <textarea id="queryEditor" spellcheck="false"></textarea>
            </div>
            <div class="panel">
              <h2>Variables JSON</h2>
              <textarea id="variablesEditor" spellcheck="false">{}</textarea>
            </div>
          </div>
          <div class="panel">
            <div class="response-header">
              <h2>Response viewer</h2>
              <span id="responseStatus" class="badge">No request yet</span>
            </div>
            <pre id="responseViewer"></pre>
          </div>
        </section>

        <section id="screen-tenant" class="screen">
          <div class="panel">
            <h2>Tenant builder</h2>
            <p class="help">A tenant is an isolation boundary. Use tenants to group related entities, resources, groups, roles, and policies.</p>
            <div class="grid">
              <label>Name<input id="tenantName" value="factory-a" /></label>
              <label>Route<input id="tenantRoute" value="factory-a" /></label>
            </div>
            <label>Attributes JSON<textarea id="tenantAttributes" spellcheck="false">{}</textarea></label>
            <div class="actions">
              <button id="generateTenant">Generate createTenant</button>
              <button class="primary" id="runTenant">Run createTenant</button>
            </div>
          </div>
        </section>

        <section id="screen-profile" class="screen">
          <div class="panel">
            <h2>Profile builder</h2>
            <p class="help">A profile defines a user/domain subtype and schema. For entity profiles, <code>kind</code> is the internal Atom entity kind such as device or service, while <code>key</code> is the subtype such as client, gateway, or meter.</p>
            <div class="grid">
              <label>Tenant ID<input id="profileTenantId" placeholder="optional tenant uuid" /></label>
              <label>Object kind<input id="profileObjectKind" value="entity" /></label>
              <label>Internal kind<select id="profileKind"><option>human</option><option selected>device</option><option>service</option><option>workload</option><option>application</option></select></label>
              <label>Profile key<input id="profileKey" value="client" /></label>
              <label>Display name<input id="profileDisplayName" value="Client" /></label>
              <label>Status<input id="profileStatus" value="active" /></label>
              <label>Description<input id="profileDescription" placeholder="optional description" /></label>
            </div>
            <div class="actions">
              <button id="generateProfile">Generate createProfile</button>
              <button class="primary" id="runProfile">Run createProfile</button>
            </div>
            <h3>Profile version</h3>
            <p class="help">Profile versions hold JSON Schema for validation and history. They are not used for authorization decisions.</p>
            <div class="grid">
              <label>Profile ID<input id="profileVersionProfileId" placeholder="profile uuid, or paste from createProfile result" /></label>
              <label>Version<input id="profileVersionNumber" type="number" value="1" /></label>
              <label>Status<input id="profileVersionStatus" value="active" /></label>
            </div>
            <label>JSON Schema<textarea id="profileJsonSchema" spellcheck="false">{"type":"object","properties":{"serial_no":{"type":"string"}}}</textarea></label>
            <label>UI Schema<textarea id="profileUiSchema" spellcheck="false">{}</textarea></label>
            <div class="actions">
              <button id="generateProfileVersion">Generate createProfileVersion</button>
              <button class="primary" id="runProfileVersion">Run createProfileVersion</button>
              <button id="refreshProfilesAfterProfile">Refresh entity profiles</button>
            </div>
            <pre id="profileResult"></pre>
          </div>
        </section>

        <section id="screen-entity" class="screen">
          <div class="panel">
            <h2>Entity builder</h2>
            <p class="help">An entity is a principal. Choose a profile when the user/domain subtype matters. If a profile is selected, do not choose kind manually: Atom derives internal runtime/authz kind from the profile.</p>
            <div class="actions">
              <button id="loadProfiles">Load entity profiles</button>
            </div>
            <div class="grid">
              <label>Profile grouped by kind<select id="entityProfile"></select></label>
              <label>Profile version<select id="entityProfileVersion"></select></label>
              <label>Name<input id="entityName" placeholder="entity-001" /></label>
              <label>Tenant ID<input id="entityTenantId" placeholder="optional tenant uuid" /></label>
            </div>
            <h3>Attributes from JSON Schema</h3>
            <div id="schemaForm" class="grid"></div>
            <label>Attributes JSON fallback<textarea id="entityAttributes" spellcheck="false">{}</textarea></label>
            <div class="actions">
              <button id="generateEntity">Generate createEntity</button>
              <button class="primary" id="runEntity">Run createEntity</button>
            </div>
            <pre id="entityResult"></pre>
          </div>
        </section>

        <section id="screen-resource" class="screen">
          <div class="panel">
            <h2>Resource builder</h2>
            <p class="help">A resource is any protected object. A channel-like object is modeled generically with <code>kind = "channel"</code>; there is no special channel mutation.</p>
            <div class="grid">
              <label>Kind<input id="resourceKind" value="channel" /></label>
              <label>Name<input id="resourceName" value="telemetry" /></label>
              <label>Tenant ID<input id="resourceTenantId" placeholder="optional tenant uuid" /></label>
              <label>Owner ID<input id="resourceOwnerId" placeholder="optional entity uuid" /></label>
            </div>
            <label>Attributes JSON<textarea id="resourceAttributes" spellcheck="false">{"topic":"telemetry"}</textarea></label>
            <div class="actions">
              <button id="generateResource">Generate createResource</button>
              <button class="primary" id="runResource">Run createResource</button>
            </div>
          </div>
        </section>

        <section id="screen-policy" class="screen">
          <div class="panel">
            <h2>Policy builder</h2>
            <p class="help">Policies grant a capability or role to a subject over a scope. Publish/subscribe connections can be modeled with generic policies over resource ids.</p>
            <div class="grid">
              <label>Tenant ID<input id="policyTenantId" placeholder="optional tenant uuid" /></label>
              <label>Subject kind<select id="policySubjectKind"><option>entity</option><option>group</option></select></label>
              <label>Subject ID<input id="policySubjectId" /></label>
              <label>Grant kind<select id="policyGrantKind"><option>capability</option><option>role</option></select></label>
              <label>Grant ID<input id="policyGrantId" /></label>
              <label>Scope kind<select id="policyScopeKind"><option>platform</option><option>tenant</option><option>object_kind</option><option>object_type</option><option selected>object</option></select></label>
              <label>Scope ref<input id="policyScopeRef" placeholder="resource uuid for object scope" /></label>
              <label>Effect<select id="policyEffect"><option selected>allow</option><option>deny</option></select></label>
            </div>
            <label>Conditions JSON<textarea id="policyConditions" spellcheck="false">{}</textarea></label>
            <div class="notice">Subject is usually an entity or group. Object scope can point to a resource id. This builder uses generic Atom naming only.</div>
            <div class="actions">
              <button id="generatePolicy">Generate createPolicy</button>
              <button class="primary" id="runPolicy">Run createPolicy</button>
            </div>
          </div>
        </section>

        <section id="screen-authz" class="screen">
          <div class="panel">
            <h2>Authz builder</h2>
            <p class="help">Use authzCheck for a normal policy decision. authzExplain returns decision details and may require privileged permissions.</p>
            <div class="grid">
              <label>Subject ID<input id="authzSubjectId" /></label>
              <label>Action<input id="authzAction" value="publish" /></label>
              <label>Resource ID<input id="authzResourceId" /></label>
              <label>Object kind<input id="authzObjectKind" placeholder="optional object kind" /></label>
              <label>Object ID<input id="authzObjectId" placeholder="optional object uuid" /></label>
            </div>
            <label>Context JSON<textarea id="authzContext" spellcheck="false">{}</textarea></label>
            <div class="notice">If authzExplain returns forbidden, use authzCheck or login as a user with policy management permissions.</div>
            <div class="actions">
              <button id="generateAuthzCheck">Generate authzCheck</button>
              <button id="generateAuthzExplain">Generate authzExplain</button>
              <button class="primary" id="runAuthz">Run current authz mutation</button>
            </div>
          </div>
        </section>

        <section id="screen-assistant" class="screen">
          <div class="panel">
            <h2>AI Assistant placeholder</h2>
            <p class="help">LLM execution is not wired yet. Copy this prompt into your AI tool.</p>
            <label>Describe what you want<textarea id="assistantRequest" spellcheck="false" placeholder="Example: create a device from the client profile and let it publish to a channel resource"></textarea></label>
            <div class="actions">
              <button id="generatePrompt">Generate prompt</button>
              <button id="copyPrompt">Copy prompt</button>
            </div>
            <pre id="assistantPrompt"></pre>
          </div>
        </section>
      </div>
    </main>

    <aside class="docs-panel">
      <div class="docs-scroll">
        <div class="brand">
          <h1>Schema Docs</h1>
          <div class="subtitle">Plain-language Atom model reference.</div>
        </div>
        <section class="nav-section">
          <dl class="schema-docs">
            <dt>Tenant</dt><dd>Isolation boundary.</dd>
            <dt>Entity</dt><dd>Principal; human/device/service/workload/application.</dd>
            <dt>Resource</dt><dd>Protected object, for example channel/rule/report.</dd>
            <dt>Group</dt><dd>Collection of entities.</dd>
            <dt>Profile</dt><dd>User/domain subtype/schema.</dd>
            <dt>ProfileVersion</dt><dd>JSON Schema validation/history.</dd>
            <dt>Policy</dt><dd>Grants capability/role over scope.</dd>
          </dl>
        </section>
        <section class="nav-section">
          <h2>Generic external mapping</h2>
          <p class="help">External systems should use generic Atom operations:</p>
          <ul class="help">
            <li>domain -> createTenant</li>
            <li>client -> createEntity with profile client under kind device</li>
            <li>channel -> createResource with kind "channel"</li>
            <li>connection -> createPolicy for publish/subscribe</li>
          </ul>
          <p class="help">Do not add GraphQL aliases for these.</p>
        </section>
        <section class="nav-section">
          <h2>Types</h2>
          <div id="objectTypes" class="type-list"></div>
          <h3>Input types</h3>
          <div id="inputTypes" class="type-list"></div>
          <h3>Enums</h3>
          <div id="enumTypes" class="type-list"></div>
        </section>
      </div>
    </aside>
  </div>

  <script>
    const state = {
      schema: null,
      typeMap: new Map(),
      selectedOperation: null,
      lastResult: null
    };

    const introspectionQuery = `
query IntrospectionQuery {
  __schema {
    queryType { name }
    mutationType { name }
    types { ...FullType }
  }
}

fragment FullType on __Type {
  kind
  name
  description
  fields(includeDeprecated: true) {
    name
    description
    args { ...InputValue }
    type { ...TypeRef }
    isDeprecated
    deprecationReason
  }
  inputFields { ...InputValue }
  enumValues(includeDeprecated: true) {
    name
    description
    isDeprecated
    deprecationReason
  }
}

fragment InputValue on __InputValue {
  name
  description
  type { ...TypeRef }
  defaultValue
}

fragment TypeRef on __Type {
  kind
  name
  ofType {
    kind
    name
    ofType {
      kind
      name
      ofType {
        kind
        name
        ofType {
          kind
          name
          ofType {
            kind
            name
            ofType {
              kind
              name
            }
          }
        }
      }
    }
  }
}`;

    const $ = (id) => document.getElementById(id);

    function endpoint() {
      const value = $("endpoint").value.trim() || "/graphql";
      if (value !== "/graphql") {
        throw new Error("For token safety, this console sends requests only to /graphql.");
      }
      return value;
    }

    function authHeaders() {
      const token = $("token").value.trim();
      return token ? { Authorization: `Bearer ${token}` } : {};
    }

    async function requestGraphql(query, variables = {}, useAuth = true) {
      const res = await fetch(endpoint(), {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          ...(useAuth ? authHeaders() : {})
        },
        body: JSON.stringify({ query, variables })
      });
      return res.json();
    }

    function unwrapType(type) {
      let node = type;
      while (node && node.ofType) node = node.ofType;
      return node;
    }

    function typeName(type) {
      if (!type) return "";
      if (type.kind === "NON_NULL") return `${typeName(type.ofType)}!`;
      if (type.kind === "LIST") return `[${typeName(type.ofType)}]`;
      return type.name || "";
    }

    function namedType(type) {
      return unwrapType(type)?.name;
    }

    function isNonNull(type) {
      return type?.kind === "NON_NULL";
    }

    function baseType(type) {
      return state.typeMap.get(namedType(type));
    }

    function escapeHtml(value) {
      return String(value).replace(/[&<>"']/g, (char) => ({
        "&": "&amp;",
        "<": "&lt;",
        ">": "&gt;",
        "\"": "&quot;",
        "'": "&#39;"
      })[char]);
    }

    function defaultValueForType(type) {
      const named = namedType(type);
      if (!isNonNull(type)) return null;
      if (["String", "ID"].includes(named)) return "";
      if (["Int", "Float"].includes(named)) return 0;
      if (named === "Boolean") return false;
      if (named === "JSON" || named === "JSONObject") return {};
      const gqlType = state.typeMap.get(named);
      if (gqlType?.kind === "ENUM") return gqlType.enumValues?.[0]?.name || "";
      if (gqlType?.kind === "INPUT_OBJECT") {
        return Object.fromEntries((gqlType.inputFields || []).map((field) => [field.name, defaultValueForType(field.type)]));
      }
      return "";
    }

    function indent(text) {
      return text.split("\n").map((line) => `  ${line}`).join("\n");
    }

    function pascal(name) {
      return name.charAt(0).toUpperCase() + name.slice(1);
    }

    function responseFieldsFor(type) {
      const gqlType = baseType(type);
      return (gqlType?.fields || []).filter((field) => !field.name.startsWith("__"));
    }

    function defaultReturnSelection(type, depth = 0) {
      const fields = responseFieldsFor(type);
      if (!fields.length) return "";
      const scalarFields = fields.filter((field) => ["SCALAR", "ENUM"].includes(unwrapType(field.type)?.kind));
      const selected = scalarFields.length ? scalarFields : fields.slice(0, 8);
      return selected.map((field) => {
        const kind = unwrapType(field.type)?.kind;
        if (["OBJECT", "INTERFACE"].includes(kind) && depth < 1) {
          const nested = defaultReturnSelection(field.type, depth + 1);
          return nested ? `${field.name} {\n${indent(nested)}\n}` : field.name;
        }
        return field.name;
      }).join("\n");
    }

    function operationText(kind, op, selectedFields = null) {
      const args = op.args || [];
      const vars = args.map((arg) => `$${arg.name}: ${typeName(arg.type)}`).join(", ");
      const argText = args.map((arg) => `${arg.name}: $${arg.name}`).join(", ");
      const selection = selectedFields ? selectedFields.join("\n") : defaultReturnSelection(op.type);
      const header = `${kind} ${pascal(op.name)}${vars ? `(${vars})` : ""}`;
      const call = `${op.name}${argText ? `(${argText})` : ""}`;
      if (!selection) return `${header} {\n  ${call}\n}`;
      return `${header} {\n  ${call} {\n${indent(indent(selection))}\n  }\n}`;
    }

    function variableSkeleton(op) {
      return Object.fromEntries((op.args || []).map((arg) => [arg.name, defaultValueForType(arg.type)]));
    }

    function requiredText(arg) {
      return isNonNull(arg.type) ? "required" : "optional";
    }

    function operationMatches(op, search) {
      if (!search) return true;
      const haystack = [
        op.name,
        typeName(op.type),
        ...(op.args || []).map((arg) => `${arg.name} ${typeName(arg.type)}`)
      ].join(" ").toLowerCase();
      return haystack.includes(search.toLowerCase());
    }

    function renderOperationList(targetId, kind, rootTypeName) {
      const root = state.typeMap.get(rootTypeName);
      const target = $(targetId);
      const search = $("schemaSearch").value.trim();
      target.innerHTML = "";

      for (const op of (root?.fields || []).filter((field) => operationMatches(field, search))) {
        const button = document.createElement("button");
        button.className = "operation-button";
        const args = (op.args || []).map((arg) => `${arg.name}: ${typeName(arg.type)} (${requiredText(arg)})`).join(", ") || "no arguments";
        button.innerHTML = `<strong>${escapeHtml(op.name)}</strong><span>${escapeHtml(args)} -> ${escapeHtml(typeName(op.type))}</span>`;
        button.addEventListener("click", () => selectOperation(kind, op));
        target.appendChild(button);
      }

      if (!target.children.length) {
        target.innerHTML = `<span class="help">No ${kind} operations match the current search.</span>`;
      }
    }

    function selectOperation(kind, op) {
      state.selectedOperation = { kind, op };
      const args = (op.args || []).map((arg) => `<span class="badge ${isNonNull(arg.type) ? "warn" : ""}">${escapeHtml(arg.name)}: ${escapeHtml(typeName(arg.type))} ${requiredText(arg)}</span>`).join(" ");
      $("selectedOperation").innerHTML = `<strong>${kind} ${escapeHtml(op.name)}</strong><div class="status-row" style="margin-top: 8px;">${args || '<span class="badge">No arguments</span>'}<span class="badge">returns ${escapeHtml(typeName(op.type))}</span></div>`;
      $("queryEditor").value = operationText(kind, op);
      $("variablesEditor").value = JSON.stringify(variableSkeleton(op), null, 2);
      renderReturnFieldSelector(op);
      showScreen("explorer");
    }

    function renderReturnFieldSelector(op) {
      const target = $("returnFields");
      const fields = responseFieldsFor(op.type);
      target.innerHTML = "";
      target.classList.toggle("hidden", !fields.length);
      if (!fields.length) return;
      const defaults = new Set(defaultReturnSelection(op.type).split(/\s+/).filter(Boolean));
      for (const field of fields) {
        const label = document.createElement("label");
        const checked = defaults.has(field.name) ? "checked" : "";
        label.innerHTML = `<input type="checkbox" value="${escapeHtml(field.name)}" ${checked}> ${escapeHtml(field.name)}`;
        label.querySelector("input").addEventListener("change", regenerateSelection);
        target.appendChild(label);
      }
    }

    function regenerateSelection() {
      const current = state.selectedOperation;
      if (!current) return;
      const selected = Array.from($("returnFields").querySelectorAll("input:checked")).map((input) => input.value);
      if (selected.length) $("queryEditor").value = operationText(current.kind, current.op, selected);
    }

    function typeMatches(type, search) {
      if (!search) return true;
      return type.name?.toLowerCase().includes(search.toLowerCase());
    }

    function renderTypeLists() {
      const search = $("schemaSearch").value.trim();
      const types = Array.from(state.typeMap.values()).filter((type) => type.name && !type.name.startsWith("__") && typeMatches(type, search));
      $("objectTypes").innerHTML = types.filter((type) => type.kind === "OBJECT").map((type) => `<span class="pill">${escapeHtml(type.name)}</span>`).join("");
      $("inputTypes").innerHTML = types.filter((type) => type.kind === "INPUT_OBJECT").map((type) => `<span class="pill">${escapeHtml(type.name)}</span>`).join("");
      $("enumTypes").innerHTML = types.filter((type) => type.kind === "ENUM").map((type) => `<span class="pill">${escapeHtml(type.name)}</span>`).join("");
    }

    function renderSchema() {
      if (!state.schema) return;
      renderOperationList("queryOps", "query", state.schema.queryType.name);
      renderOperationList("mutationOps", "mutation", state.schema.mutationType.name);
      renderTypeLists();
    }

    function updateStatus() {
      const schemaLoaded = Boolean(state.schema);
      const tokenPresent = Boolean($("token").value.trim());
      $("stepEndpoint").classList.add("done");
      $("stepSchema").classList.toggle("done", schemaLoaded);
      $("stepLogin").classList.toggle("done", tokenPresent);
      $("connectionStatus").innerHTML = `
        <span class="badge ok">Endpoint /graphql</span>
        <span class="badge ${schemaLoaded ? "ok" : "warn"}">${schemaLoaded ? "Schema loaded" : "Schema not loaded"}</span>
        <span class="badge ${tokenPresent ? "ok" : "warn"}">${tokenPresent ? "Token ready" : "Not authenticated"}</span>
      `;
    }

    async function loadSchema() {
      $("schemaStatus").innerHTML = `<span class="badge warn">Loading schema</span>`;
      updateStatus();
      try {
        const result = await requestGraphql(introspectionQuery, {}, false);
        if (result.errors?.length) throw new Error(result.errors.map((err) => err.message).join("; "));
        state.schema = result.data.__schema;
        state.typeMap = new Map(state.schema.types.map((type) => [type.name, type]));
        renderSchema();
        $("schemaStatus").innerHTML = `<span class="badge ok">Loaded ${state.schema.types.length} types</span>`;
      } catch (err) {
        $("schemaStatus").innerHTML = `<span class="badge error">${escapeHtml(err.message)}</span>`;
      }
      updateStatus();
    }

    function parseJson(id) {
      const text = $(id).value.trim();
      if (!text) return {};
      try {
        return JSON.parse(text);
      } catch (err) {
        throw new Error(`${id} contains invalid JSON: ${err.message}`);
      }
    }

    function setOperation(query, variables, openExplorer = true) {
      $("queryEditor").value = query;
      $("variablesEditor").value = JSON.stringify(variables, null, 2);
      if (openExplorer) showScreen("explorer");
    }

    function writeResponse(target, result) {
      target.textContent = JSON.stringify(result, null, 2);
      state.lastResult = result;
      $("responseStatus").className = result.errors?.length ? "badge error" : "badge ok";
      $("responseStatus").textContent = result.errors?.length ? "GraphQL error" : "Success";
    }

    function showConsoleError(message, targetId = "responseViewer") {
      const target = $(targetId);
      target.textContent = message;
      if (targetId === "responseViewer") {
        $("responseStatus").className = "badge error";
        $("responseStatus").textContent = "Input error";
        showScreen("explorer");
      }
    }

    function generateSafely(generator, targetId = "responseViewer") {
      try {
        generator();
      } catch (err) {
        showConsoleError(err.message, targetId);
      }
    }

    async function runGenerated(generator, targetId = "responseViewer") {
      try {
        generator(targetId === "responseViewer");
        return await runCurrent(targetId);
      } catch (err) {
        showConsoleError(err.message, targetId);
        return false;
      }
    }

    async function runCurrent(targetId = "responseViewer") {
      const target = $(targetId);
      try {
        const result = await requestGraphql($("queryEditor").value, parseJson("variablesEditor"));
        writeResponse(target, result);
        return !result.errors?.length;
      } catch (err) {
        showConsoleError(err.message, targetId);
        return false;
      }
    }

    function loginMutation() {
      return `mutation Login($input: LoginInput!) {
  login(input: $input) {
    token
    entityId
    sessionId
    expiresAt
  }
}`;
    }

    async function runLogin() {
      const variables = {
        input: {
          identifier: $("loginIdentifier").value,
          secret: $("loginSecret").value,
          kind: $("loginKind").value || "password"
        }
      };
      const result = await requestGraphql(loginMutation(), variables, false);
      $("loginResult").textContent = JSON.stringify(result, null, 2);
      const login = result.data?.login;
      if (login?.token) {
        $("token").value = login.token;
        localStorage.setItem("atom.graphql.console.token", login.token);
        $("loginResult").textContent += `\n\nentityId: ${login.entityId}\nsessionId: ${login.sessionId}\nexpiresAt: ${login.expiresAt}`;
      }
      updateStatus();
    }

    function clearToken() {
      $("token").value = "";
      localStorage.removeItem("atom.graphql.console.token");
      updateStatus();
    }

    async function loadProfiles() {
      const query = `query EntityProfiles {
  profiles(objectKind: "entity", limit: 200) {
    items {
      id
      kind
      key
      displayName
    }
  }
}`;
      const result = await requestGraphql(query, {});
      if (result.errors?.length) throw new Error(result.errors.map((err) => err.message).join("; "));
      const profiles = result.data?.profiles?.items || [];
      const select = $("entityProfile");
      select.innerHTML = "";
      for (const profile of profiles.sort((a, b) => `${a.kind}:${a.key}`.localeCompare(`${b.kind}:${b.key}`))) {
        const option = document.createElement("option");
        option.value = profile.id;
        option.textContent = `${profile.kind} / ${profile.key} - ${profile.displayName}`;
        option.dataset.kind = profile.kind;
        select.appendChild(option);
      }
      if (!profiles.length) select.innerHTML = `<option value="">No entity profiles returned</option>`;
      await loadProfileVersions();
    }

    async function loadProfileVersions() {
      const profileId = $("entityProfile").value;
      if (!profileId) return;
      const query = `query ProfileVersions($profileId: ID!) {
  profileVersions(profileId: $profileId) {
    id
    version
    jsonSchema
    status
  }
}`;
      const result = await requestGraphql(query, { profileId });
      if (result.errors?.length) throw new Error(result.errors.map((err) => err.message).join("; "));
      const versions = result.data?.profileVersions || [];
      const select = $("entityProfileVersion");
      select.innerHTML = "";
      for (const version of versions) {
        const option = document.createElement("option");
        option.value = version.id;
        option.textContent = `v${version.version} ${version.status}`;
        option.dataset.schema = JSON.stringify(version.jsonSchema || {});
        option.dataset.status = version.status;
        select.appendChild(option);
      }
      const activeIndex = Array.from(select.options).findIndex((option) => option.dataset.status === "active");
      select.selectedIndex = activeIndex >= 0 ? activeIndex : 0;
      renderJsonSchemaForm();
    }

    function renderJsonSchemaForm() {
      const option = $("entityProfileVersion").selectedOptions[0];
      const schema = option?.dataset.schema ? JSON.parse(option.dataset.schema) : {};
      const props = schema.properties || {};
      const form = $("schemaForm");
      form.innerHTML = "";
      for (const [name, spec] of Object.entries(props)) {
        const label = document.createElement("label");
        label.textContent = name;
        const input = document.createElement("input");
        input.dataset.attr = name;
        input.type = spec.type === "number" || spec.type === "integer" ? "number" : "text";
        input.placeholder = spec.description || spec.type || "";
        label.appendChild(input);
        form.appendChild(label);
      }
      if (!Object.keys(props).length) {
        form.innerHTML = `<div class="help">This profile version has no simple JSON Schema properties. Use the JSON fallback below.</div>`;
      }
    }

    function schemaFormAttributes() {
      const attrs = {};
      for (const input of $("schemaForm").querySelectorAll("[data-attr]")) {
        if (input.value === "") continue;
        attrs[input.dataset.attr] = input.type === "number" ? Number(input.value) : input.value;
      }
      return Object.keys(attrs).length ? attrs : parseJson("entityAttributes");
    }

    function entityMutation() {
      return `mutation CreateEntity($input: CreateEntityInput!) {
  createEntity(input: $input) {
    id
    kind
    profileId
    profileVersionId
    name
    tenantId
    attributes
  }
}`;
    }

    function tenantMutation() {
      return `mutation CreateTenant($input: CreateTenantInput!) {
  createTenant(input: $input) {
    id
    name
    route
    status
    attributes
  }
}`;
    }

    function generateTenant(openExplorer = true) {
      setOperation(tenantMutation(), {
        input: {
          name: $("tenantName").value,
          route: $("tenantRoute").value || null,
          attributes: parseJson("tenantAttributes")
        }
      }, openExplorer);
    }

    function profileMutation() {
      return `mutation CreateProfile($input: CreateProfileInput!) {
  createProfile(input: $input) {
    id
    tenantId
    objectKind
    kind
    key
    displayName
    description
    status
  }
}`;
    }

    function profileVersionMutation() {
      return `mutation CreateProfileVersion($profileId: ID!, $input: CreateProfileVersionInput!) {
  createProfileVersion(profileId: $profileId, input: $input) {
    id
    profileId
    version
    jsonSchema
    uiSchema
    status
  }
}`;
    }

    function generateProfile(openExplorer = true) {
      setOperation(profileMutation(), {
        input: {
          tenantId: $("profileTenantId").value || null,
          objectKind: $("profileObjectKind").value || "entity",
          kind: $("profileKind").value,
          key: $("profileKey").value,
          displayName: $("profileDisplayName").value,
          description: $("profileDescription").value || null,
          status: $("profileStatus").value || null
        }
      }, openExplorer);
    }

    function generateProfileVersion(openExplorer = true) {
      setOperation(profileVersionMutation(), {
        profileId: $("profileVersionProfileId").value,
        input: {
          version: Number($("profileVersionNumber").value || 1),
          jsonSchema: parseJson("profileJsonSchema"),
          uiSchema: parseJson("profileUiSchema"),
          status: $("profileVersionStatus").value || null
        }
      }, openExplorer);
    }

    async function runProfileBuilder() {
      const ok = await runGenerated(generateProfile, "profileResult");
      if (!ok) return;
      const profileId = state.lastResult?.data?.createProfile?.id;
      if (profileId) {
        $("profileVersionProfileId").value = profileId;
        $("profileResult").textContent += `\n\nProfile ID copied into Profile version. Create a version next so Entity builder can use the schema.`;
      }
    }

    async function runProfileVersionBuilder() {
      const ok = await runGenerated(generateProfileVersion, "profileResult");
      if (ok) {
        await refreshEntityProfilesFromProfileBuilder();
      }
    }

    async function refreshEntityProfilesFromProfileBuilder() {
      try {
        await loadProfiles();
        $("profileResult").textContent += `\n\nEntity profiles refreshed. Open Entity builder to create an entity from this profile.`;
      } catch (err) {
        $("profileResult").textContent += `\n\nCould not refresh entity profiles: ${err.message}`;
      }
    }

    function generateEntity(openExplorer = true) {
      const input = {
        profileId: $("entityProfile").value || "",
        profileVersionId: $("entityProfileVersion").value || null,
        name: $("entityName").value || "",
        tenantId: $("entityTenantId").value || null,
        attributes: schemaFormAttributes()
      };
      if (!input.profileVersionId) delete input.profileVersionId;
      setOperation(entityMutation(), { input }, openExplorer);
    }

    function resourceMutation() {
      return `mutation CreateResource($input: CreateResourceInput!) {
  createResource(input: $input) {
    id
    kind
    name
    tenantId
    ownerId
    attributes
  }
}`;
    }

    function generateResource(openExplorer = true) {
      setOperation(resourceMutation(), {
        input: {
          kind: $("resourceKind").value,
          name: $("resourceName").value || null,
          tenantId: $("resourceTenantId").value || null,
          ownerId: $("resourceOwnerId").value || null,
          attributes: parseJson("resourceAttributes")
        }
      }, openExplorer);
    }

    function policyMutation() {
      return `mutation CreatePolicy($input: CreatePolicyInput!) {
  createPolicy(input: $input) {
    id
    tenantId
    subjectKind
    subjectId
    grantKind
    grantId
    scopeKind
    scopeRef
    effect
    conditions
  }
}`;
    }

    function generatePolicy(openExplorer = true) {
      setOperation(policyMutation(), {
        input: {
          tenantId: $("policyTenantId").value || null,
          subjectKind: $("policySubjectKind").value,
          subjectId: $("policySubjectId").value,
          grantKind: $("policyGrantKind").value,
          grantId: $("policyGrantId").value,
          scopeKind: $("policyScopeKind").value,
          scopeRef: $("policyScopeRef").value || null,
          effect: $("policyEffect").value,
          conditions: parseJson("policyConditions")
        }
      }, openExplorer);
    }

    function authzMutation(name) {
      return `mutation ${pascal(name)}($input: AuthzCheckInput!) {
  ${name}(input: $input) {
    allowed
    reason
    ${name === "authzExplain" ? "evaluatedBindings" : "details"}
  }
}`;
    }

    function generateAuthz(name, openExplorer = true) {
      setOperation(authzMutation(name), {
        input: {
          subjectId: $("authzSubjectId").value,
          action: $("authzAction").value,
          resourceId: $("authzResourceId").value || null,
          objectKind: $("authzObjectKind").value || null,
          objectId: $("authzObjectId").value || null,
          context: parseJson("authzContext")
        }
      }, openExplorer);
    }

    function renderSavedExamples() {
      const examples = JSON.parse(localStorage.getItem("atom.graphql.console.examples") || "{}");
      const select = $("savedExamples");
      select.innerHTML = Object.keys(examples).map((name) => `<option>${escapeHtml(name)}</option>`).join("");
    }

    function saveExample() {
      const name = prompt("Example name");
      if (!name) return;
      const examples = JSON.parse(localStorage.getItem("atom.graphql.console.examples") || "{}");
      examples[name] = { query: $("queryEditor").value, variables: $("variablesEditor").value };
      localStorage.setItem("atom.graphql.console.examples", JSON.stringify(examples));
      renderSavedExamples();
    }

    function loadExample() {
      const examples = JSON.parse(localStorage.getItem("atom.graphql.console.examples") || "{}");
      const selected = examples[$("savedExamples").value];
      if (!selected) return;
      $("queryEditor").value = selected.query;
      $("variablesEditor").value = selected.variables;
      showScreen("explorer");
    }

    function generatePrompt() {
      const queries = (state.typeMap.get(state.schema?.queryType?.name)?.fields || []).map((field) => field.name);
      const mutations = (state.typeMap.get(state.schema?.mutationType?.name)?.fields || []).map((field) => field.name);
      const enums = Array.from(state.typeMap.values()).filter((type) => type.kind === "ENUM").map((type) => `${type.name}: ${(type.enumValues || []).map((item) => item.name).join(", ")}`);
      const selected = state.selectedOperation ? `${state.selectedOperation.kind} ${state.selectedOperation.op.name}` : "none";
      $("assistantPrompt").textContent = `You are helping generate generic Atom GraphQL.\n\nSchema summary:\nQueries: ${queries.join(", ")}\nMutations: ${mutations.join(", ")}\nEnums:\n${enums.join("\n")}\n\nSelected operation: ${selected}\n\nUser request:\n${$("assistantRequest").value}\n\nGenerate a GraphQL query or mutation and variables JSON. Use generic Atom operations only. Do not invent external-system aliases.`;
    }

    function showScreen(name) {
      document.querySelectorAll(".screen").forEach((screen) => screen.classList.toggle("active", screen.id === `screen-${name}`));
      document.querySelectorAll(".nav-button").forEach((button) => button.classList.toggle("active", button.dataset.screen === name));
    }

    document.querySelectorAll("[data-screen]").forEach((button) => button.addEventListener("click", () => showScreen(button.dataset.screen)));
    $("refreshSchema").addEventListener("click", loadSchema);
    $("schemaSearch").addEventListener("input", renderSchema);
    $("token").addEventListener("input", updateStatus);
    $("copyQuery").addEventListener("click", () => navigator.clipboard.writeText($("queryEditor").value));
    $("runOperation").addEventListener("click", () => runCurrent());
    $("runLogin").addEventListener("click", runLogin);
    $("clearToken").addEventListener("click", clearToken);
    $("loadProfiles").addEventListener("click", async () => {
      try { await loadProfiles(); } catch (err) { $("entityResult").textContent = err.message; }
    });
    $("generateTenant").addEventListener("click", () => generateSafely(generateTenant));
    $("runTenant").addEventListener("click", () => runGenerated(generateTenant));
    $("generateProfile").addEventListener("click", () => generateSafely(generateProfile, "profileResult"));
    $("runProfile").addEventListener("click", runProfileBuilder);
    $("generateProfileVersion").addEventListener("click", () => generateSafely(generateProfileVersion, "profileResult"));
    $("runProfileVersion").addEventListener("click", runProfileVersionBuilder);
    $("refreshProfilesAfterProfile").addEventListener("click", refreshEntityProfilesFromProfileBuilder);
    $("entityProfile").addEventListener("change", loadProfileVersions);
    $("entityProfileVersion").addEventListener("change", renderJsonSchemaForm);
    $("generateEntity").addEventListener("click", () => generateSafely(generateEntity, "entityResult"));
    $("runEntity").addEventListener("click", () => runGenerated(generateEntity, "entityResult"));
    $("generateResource").addEventListener("click", () => generateSafely(generateResource));
    $("runResource").addEventListener("click", () => runGenerated(generateResource));
    $("generatePolicy").addEventListener("click", () => generateSafely(generatePolicy));
    $("runPolicy").addEventListener("click", () => runGenerated(generatePolicy));
    $("generateAuthzCheck").addEventListener("click", () => generateSafely(() => generateAuthz("authzCheck")));
    $("generateAuthzExplain").addEventListener("click", () => generateSafely(() => generateAuthz("authzExplain")));
    $("runAuthz").addEventListener("click", () => runCurrent());
    $("saveExample").addEventListener("click", saveExample);
    $("loadExample").addEventListener("click", loadExample);
    $("generatePrompt").addEventListener("click", generatePrompt);
    $("copyPrompt").addEventListener("click", () => navigator.clipboard.writeText($("assistantPrompt").textContent));

    const storedToken = localStorage.getItem("atom.graphql.console.token");
    if (storedToken) $("token").value = storedToken;
    renderSavedExamples();
    updateStatus();
    loadSchema();
  </script>
</body>
</html>
"###
}

#[cfg(test)]
mod tests {
    use super::console_html;

    #[test]
    fn console_html_contains_expected_sections() {
        let html = console_html();

        for text in [
            "Atom GraphQL Console",
            "Getting Started",
            "operation explorer",
            "Login helper",
            "Tenant builder",
            "Profile builder",
            "Entity builder",
            "Resource builder",
            "Policy builder",
            "Authz builder",
            "AI Assistant placeholder",
            "Schema Docs",
        ] {
            assert!(html.contains(text), "missing {text}");
        }

        assert!(html.contains("function typeName(type)"));
        assert!(html.contains("function renderOperationList(targetId, kind, rootTypeName)"));
        assert!(html.contains("For token safety, this console sends requests only to /graphql."));
        assert!(html.contains("mutation CreateProfile($input: CreateProfileInput!)"));
        assert!(html.contains(
            "mutation CreateProfileVersion($profileId: ID!, $input: CreateProfileVersionInput!)"
        ));
    }

    #[test]
    fn console_html_uses_generic_atom_operations_only() {
        let html = console_html();

        for suffix in ["Domain", "Client", "Channel"] {
            assert!(!html.contains(&format!("create{suffix}")));
        }
    }
}
