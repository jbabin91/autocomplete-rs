# Fig Completion Spec Format Research

**Research Date:** 2026-02-06
**Target Repository:** [withfig/autocomplete](https://github.com/withfig/autocomplete)
**Purpose:** Inform autocomplete-rs parser design and implementation

---

## Executive Summary

The Fig completion spec format is a **TypeScript-based declarative schema** for defining CLI autocomplete behavior. The withfig/autocomplete repository contains **500+ completion specs** for popular CLI tools, written entirely in TypeScript and compiled to JavaScript for runtime execution.

**Key Findings:**

- **Spec Structure:** Hierarchical tree of `Spec` → `Subcommand` → `Option` → `Arg` objects
- **Generator System:** Three types (script, custom, template) enable dynamic completions via shell execution
- **Lazy Loading:** `loadSpec` property enables on-demand loading of nested specs (critical for large CLIs like AWS)
- **Complexity Range:** From 50-line simple utilities to 3,500+ line complex tools
- **Build Process:** TypeScript compiled to JavaScript via `@withfig/autocomplete-tools`
- **License:** MIT

---

## 1. Spec Structure

### 1.1 Top-Level Type Hierarchy

The Fig namespace defines these core types:

```typescript
Fig.Spec; // Root object (actually a Subcommand)
Fig.Subcommand; // Nested commands
Fig.Option; // Flags and options (-m, --message)
Fig.Arg; // Command arguments
Fig.Generator; // Dynamic suggestion generation
Fig.Suggestion; // Base class for autocomplete items
```

**Inheritance:** `Subcommand` and `Option` extend `Suggestion`, inheriting display properties like `name`, `description`, `icon`, and `priority`.

### 1.2 Minimal Spec Example

From the `cd` command spec:

```typescript
const completionSpec: Fig.Spec = {
  name: 'cd',
  description: 'Change the shell working directory',
  args: {
    name: 'directory',
    description: 'The directory to change to',
    generators: filepaths({
      showFolders: 'only',
    }),
    filterStrategy: 'fuzzy',
    suggestions: [
      {
        name: '-',
        description: 'Switch to previous directory',
        hidden: true,
      },
      {
        name: '~',
        description: 'Home directory',
        hidden: true,
      },
    ],
  },
};

export default completionSpec;
```

**Key Observations:**

- Must export `completionSpec` as default
- `name` is required and must match the CLI tool name exactly
- Combines static suggestions with dynamic generators
- Hidden suggestions available for advanced users

### 1.3 Subcommand Nesting

**Maximum observed depth:** 3 levels (kubectl, aws, cargo)

Example from kubectl:

```text
kubectl → create → secret → docker-registry
```

Most specs stay at 1-2 levels. Deep nesting is handled via `loadSpec` for maintainability.

---

## 2. Core Type Definitions

### 2.1 Fig.Spec / Fig.Subcommand

The root spec IS a subcommand. Key properties:

| Property             | Type                               | Description                       |
| -------------------- | ---------------------------------- | --------------------------------- |
| `name`               | `string \| string[]`               | Command name(s), supports aliases |
| `description`        | `string`                           | Help text displayed in UI         |
| `subcommands`        | `Subcommand[]`                     | Nested subcommands                |
| `options`            | `Option[]`                         | Available flags and options       |
| `args`               | `Arg \| Arg[]`                     | Positional arguments              |
| `loadSpec`           | `string \| Subcommand \| function` | Lazy-load external spec           |
| `generateSpec`       | `function`                         | Programmatically generate spec    |
| `requiresSubcommand` | `boolean`                          | Enforce subcommand presence       |
| `parserDirectives`   | `object`                           | Control parsing behavior          |
| `filterStrategy`     | `"fuzzy" \| "prefix" \| "default"` | Suggestion filtering              |
| `icon`               | `string`                           | Display icon (emoji, URL, fig://) |
| `priority`           | `number` (0-100)                   | Ranking weight                    |
| `isDangerous`        | `boolean`                          | Prevent auto-execution            |
| `hidden`             | `boolean`                          | Only show on exact match          |
| `deprecated`         | `boolean \| object`                | Deprecation marker                |
| `cache`              | `object`                           | Cache strategy configuration      |

**Example with aliases and loadSpec (from git):**

```typescript
{
  name: ["checkout", "co"],
  description: "Switch branches or restore working tree files",
  args: {
    name: "branch or file",
    generators: [branchGenerator, filesGenerator],
  },
}
```

**Example with loadSpec (from AWS):**

```typescript
{
  name: "s3",
  description: "Amazon Simple Storage Service",
  loadSpec: "aws/s3",  // Loads src/aws/s3.ts on-demand
}
```

### 2.2 Fig.Option

Defines CLI flags and options.

| Property            | Type                 | Description                          |
| ------------------- | -------------------- | ------------------------------------ |
| `name`              | `string \| string[]` | Exact flag name(s) as typed          |
| `description`       | `string`             | Help text                            |
| `args`              | `Arg \| Arg[]`       | Arguments the option accepts         |
| `isRequired`        | `boolean`            | Option must be provided              |
| `isPersistent`      | `boolean`            | Available to all child subcommands   |
| `isRepeatable`      | `boolean \| number`  | Can be specified multiple times      |
| `exclusiveOn`       | `string[]`           | Mutually exclusive option names      |
| `dependsOn`         | `string[]`           | Required prerequisite options        |
| `requiresSeparator` | `boolean`            | Requires `=` separator (--key=value) |

**Example from git commit:**

```typescript
{
  name: ["-m", "--message"],
  description: "Use the given message as the commit message",
  args: {
    name: "message",
    description: "Commit message text",
  },
  isRepeatable: true,
}
```

**Example with exclusivity (from docker):**

```typescript
{
  name: "--git",
  description: "Set Git repository URL",
  args: { name: "url" },
  exclusiveOn: ["--path", "--index"],  // Can't use with --path or --index
}
```

### 2.3 Fig.Arg

Represents user-defined input values.

| Property           | Type                                              | Description                           |
| ------------------ | ------------------------------------------------- | ------------------------------------- |
| `name`             | `string`                                          | Human-readable label                  |
| `description`      | `string`                                          | Help text when no suggestions         |
| `suggestions`      | `(string \| Suggestion)[]`                        | Static suggestion list                |
| `generators`       | `Generator \| Generator[]`                        | Dynamic suggestion generation         |
| `template`         | `"filepaths" \| "folders" \| "history" \| "help"` | Built-in generators                   |
| `isVariadic`       | `boolean`                                         | Repeats infinitely                    |
| `isOptional`       | `boolean`                                         | Not required                          |
| `isDangerous`      | `boolean`                                         | Disable auto-execution                |
| `isCommand`        | `boolean`                                         | Argument is itself a command          |
| `isScript`         | `boolean`                                         | Look for spec in user's fig directory |
| `isModule`         | `string`                                          | Prepend string for spec lookup        |
| `filterStrategy`   | `"fuzzy" \| "prefix" \| "default"`                | Suggestion filtering                  |
| `debounce`         | `boolean \| number`                               | Delay generator execution             |
| `default`          | `string`                                          | Default value                         |
| `parserDirectives` | `object`                                          | Alias expansion config                |

**Example with variadic args (from docker rm):**

```typescript
{
  name: "rm",
  description: "Remove one or more containers",
  args: {
    name: "containers",
    isVariadic: true,
    generators: dockerGenerators.allDockerContainers,
  },
}
```

**Example with static suggestions (from aws s3):**

```typescript
{
  name: "--storage-class",
  args: {
    name: "class",
    suggestions: [
      "STANDARD",
      "REDUCED_REDUNDANCY",
      "STANDARD_IA",
      "ONEZONE_IA",
      "INTELLIGENT_TIERING",
      "GLACIER",
      "DEEP_ARCHIVE",
    ],
  },
}
```

### 2.4 Fig.Suggestion

Base class for autocomplete items.

| Property           | Type                                                                                            | Description                   |
| ------------------ | ----------------------------------------------------------------------------------------------- | ----------------------------- |
| `name`             | `string \| string[]`                                                                            | Used for filtering            |
| `displayName`      | `string`                                                                                        | Text shown in UI              |
| `description`      | `string`                                                                                        | Help text at bottom           |
| `icon`             | `string`                                                                                        | Visual indicator              |
| `insertValue`      | `string`                                                                                        | Text inserted on selection    |
| `replaceValue`     | `string`                                                                                        | Replaces entire buffer        |
| `type`             | `"folder" \| "file" \| "arg" \| "subcommand" \| "option" \| "special" \| "mixin" \| "shortcut"` | Determines default icon       |
| `priority`         | `number` (0-100)                                                                                | Ranking weight (default 50)   |
| `isDangerous`      | `boolean`                                                                                       | Prevent auto-execution        |
| `hidden`           | `boolean`                                                                                       | Only show on exact match      |
| `deprecated`       | `boolean \| object`                                                                             | Deprecation info              |
| `previewComponent` | `string`                                                                                        | Custom preview component path |

**Priority system:** Fig adjusts priorities based on usage. Items in 50-75 range become `75 + timestamp`, others increment by timestamp.

**InsertValue with cursor positioning:**

```typescript
{
  name: "branch.{cursor}.description",
  insertValue: "branch.{cursor}.description",
  description: "Set branch description",
}
```

### 2.5 Fig.Generator

Enables dynamic suggestion generation via shell commands or custom functions.

| Property                    | Type                                                           | Description                                          |
| --------------------------- | -------------------------------------------------------------- | ---------------------------------------------------- |
| `template`                  | `"filepaths" \| "folders" \| "history" \| "help" \| array`     | Built-in generators                                  |
| `script`                    | `string \| ((tokens: string[]) => string)`                     | Shell command to execute                             |
| `postProcess`               | `(output: string, tokens: string[]) => Suggestion[]`           | Transform script output                              |
| `splitOn`                   | `string`                                                       | Syntactic sugar: split output and create suggestions |
| `custom`                    | `async (tokens, executeShellCommand, context) => Suggestion[]` | Imperative control                                   |
| `scriptTimeout`             | `number`                                                       | Timeout in ms (default 5000)                         |
| `trigger`                   | `string \| function \| object`                                 | When to invalidate cache                             |
| `getQueryTerm`              | `string \| ((token: string) => string)`                        | Extract filterable portion                           |
| `filterTemplateSuggestions` | `(suggestions) => Suggestion[]`                                | Filter template results                              |
| `cache`                     | `object`                                                       | Caching strategy                                     |

#### Generator Types

**1. Template Generators (built-in):**

```typescript
{
  template: "filepaths",  // Shows files and folders
}

{
  template: "folders",  // Folders only
}

{
  template: ["filepaths", "history"],  // Multiple templates
}
```

**2. Script-based Generators:**

```typescript
{
  script: "git branch --list",
  postProcess: (output) => {
    return output.split("\n").map(branch => ({
      name: branch.trim(),
      description: "Git branch",
    }));
  },
}
```

**3. Dynamic Script (function):**

```typescript
{
  script: (tokens) => {
    const hasAll = tokens.includes("--all");
    return hasAll ? "git branch -a" : "git branch";
  },
  postProcess: (output) => { /* ... */ },
}
```

**4. Custom Generators (full control):**

```typescript
{
  custom: async (tokens, executeShellCommand, context) => {
    const output = await executeShellCommand("ls -1");
    return output.split("\n").map(name => ({ name }));
  },
}
```

**5. Real-world example (npm search with API call):**

```typescript
const npmSearchGenerator: Fig.Generator = {
  trigger: (newToken, oldToken) => {
    return newToken !== oldToken;
  },
  getQueryTerm: '@', // Trigger search after @ symbol
  custom: async (tokens, executeShellCommand) => {
    const searchTerm = tokens[tokens.length - 1];
    const query = searchTerm.replace('@', '');

    const cmd = `curl -s 'https://api.npms.io/v2/search?q=${query}&size=20'`;
    const output = await executeShellCommand(cmd);
    const data = JSON.parse(output);

    return data.results.map((pkg) => ({
      name: pkg.package.name,
      description: pkg.package.description,
      icon: '📦',
    }));
  },
  cache: {
    strategy: 'stale-while-revalidate',
    ttl: 1000 * 60 * 60 * 24 * 2, // 2 days
  },
};
```

#### Cache Strategies

**stale-while-revalidate (default):**

- Returns cached data immediately
- Fetches fresh data in background
- Updates cache for next request

```typescript
{
  cache: {
    strategy: "stale-while-revalidate",
    ttl: 1000 * 60 * 60,  // 1 hour
    cacheByDirectory: true,
  },
}
```

**max-age:**

- Shows loading indicator when stale
- Waits for fresh data

```typescript
{
  cache: {
    strategy: "max-age",
    ttl: 1000 * 30,  // 30 seconds
  },
}
```

**Cache key customization:**

```typescript
{
  cache: {
    cacheKey: "shared-key",  // Share cache across specs
  },
}
```

#### Trigger Configuration

Controls when to regenerate suggestions:

**On any change:**

```typescript
{
  trigger: { on: "change" },
}
```

**On threshold:**

```typescript
{
  trigger: { on: "threshold", length: 3 },  // After 3 characters
}
```

**On specific characters:**

```typescript
{
  trigger: { on: "match", string: "@" },  // When @ is typed
}
```

**Custom function:**

```typescript
{
  trigger: (newToken, oldToken) => {
    return newToken.length > 3 && newToken !== oldToken;
  },
}
```

#### getQueryTerm

Extracts the filterable portion for suggestions:

```typescript
{
  script: "ls ~/desktop",
  getQueryTerm: (token) => {
    // "cd ~/desktop/abc" → return "abc"
    return token.split("/").pop() || "";
  },
}
```

---

## 3. Real Spec Examples

### 3.1 Git Spec Analysis

**File:** `src/git.ts`
**Size:** ~1,400 lines
**Complexity:** Moderate

**Structure:**

- **250+ configuration suggestions** (`core.abbrev`, `branch.<name>.merge`, etc.)
- **Reusable generators:** commits, branches, remotes, tags, staged files
- **PostProcess patterns:** filterMessages, postProcessBranches, postProcessTrackedFiles

**Generator Example (branches):**

```typescript
const branchGenerator: Fig.Generator = {
  script: 'git branch --list',
  postProcess: (output) => {
    return output
      .split('\n')
      .map((line) => line.trim())
      .filter(Boolean)
      .map((branch) => {
        const isCurrent = branch.startsWith('*');
        const name = branch.replace('* ', '');
        return {
          name,
          description: isCurrent ? 'Current branch' : 'Branch',
          icon: isCurrent ? '⭐' : '🌿',
          priority: isCurrent ? 100 : 50,
        };
      });
  },
};
```

**Files for staging generator (complex):**

```typescript
const filesForStaging: Fig.Generator = {
  script: 'git status --porcelain',
  postProcess: (output, tokens) => {
    const includeStaged = !tokens.includes('--');

    return output.split('\n').map((line) => {
      const status = line.substring(0, 2);
      const file = line.substring(3);

      return {
        name: file,
        insertValue: (includeStaged ? '' : '-- ') + file,
        icon: 'fig://icon?type=file',
        description: `Status: ${status}`,
      };
    });
  },
};
```

### 3.2 NPM Spec Analysis

**File:** `src/npm.ts`
**Size:** ~2,000 lines
**Complexity:** High (API integration, package.json parsing)

**Structure:**

- **2-level nesting max:** `npm audit fix`, `npm config set`
- **Dynamic/Static ratio:** ~65% dynamic, 35% static
- **Key generators:** npmScripts, dependencies, npmSearch

**NPM Scripts Generator (workspace-aware):**

```typescript
const npmScriptsGenerator: Fig.Generator = {
  script: `
    until [[ -f package.json ]] || [[ $PWD = '/' ]]; do
      cd ..;
    done;
    cat package.json
  `,
  postProcess: (output) => {
    try {
      const pkg = JSON.parse(output);
      const scripts = pkg.scripts || {};
      const figCompletions = pkg.fig || {};

      return Object.keys(scripts).map((name) => ({
        name,
        description: scripts[name],
        icon: '📜',
        // Merge custom completions from fig field
        ...(figCompletions[name] || {}),
      }));
    } catch {
      return [];
    }
  },
  cache: {
    strategy: 'stale-while-revalidate',
    ttl: 1000 * 10, // 10 seconds
  },
};
```

**NPM Search with API:**

```typescript
const createNpmSearchHandler = (): Fig.Generator => ({
  custom: async (tokens, executeShellCommand) => {
    const searchTerm = tokens[tokens.length - 1] || '';
    const isVersionSearch = searchTerm.includes('@');

    if (isVersionSearch) {
      // Version search: package@^1.0.0
      const [pkg] = searchTerm.split('@');
      const cmd = `curl -s 'https://registry.npmjs.org/${pkg}'`;
      const output = await executeShellCommand(cmd);
      const data = JSON.parse(output);

      return Object.keys(data.versions || {}).map((version) => ({
        name: `${pkg}@${version}`,
        description: data.versions[version].description,
      }));
    } else {
      // Package search
      const cmd = `curl -s 'https://api.npms.io/v2/search?q=${searchTerm}&size=20'`;
      const output = await executeShellCommand(cmd);
      const data = JSON.parse(output);

      return data.results.map((result) => ({
        name: result.package.name,
        description: result.package.description,
        icon: '📦',
      }));
    }
  },
  trigger: (newToken, oldToken) => newToken !== oldToken,
  getQueryTerm: '@',
  cache: {
    strategy: 'stale-while-revalidate',
    ttl: 1000 * 60 * 60 * 24 * 2, // 2 days
  },
});
```

### 3.3 Docker Spec Analysis

**File:** `src/docker.ts`
**Size:** ~2,500 lines
**Complexity:** High (container/image management)

**Structure:**

- **Variadic args:** `docker rm <containers...>`, `docker rmi <images...>`
- **Complex arg chains:** `docker cp CONTAINER:SRC DEST | SRC CONTAINER:DEST`
- **Generator patterns:** JSON parsing from `docker ps --format`

**Container Generator:**

```typescript
const dockerGenerators = {
  allDockerContainers: {
    script: ['docker', 'ps', '-a', '--format', '{{ json . }}'],
    postProcess: (output) => {
      return output
        .split('\n')
        .filter(Boolean)
        .map((line) => JSON.parse(line))
        .map((container) => ({
          name: container.Names,
          description: `${container.Image} - ${container.Status}`,
          icon: '🐳',
        }));
    },
  },

  runningContainers: {
    script: ['docker', 'ps', '--format', '{{ json . }}'],
    postProcess: (output) => {
      return output
        .split('\n')
        .filter(Boolean)
        .map((line) => JSON.parse(line))
        .map((container) => ({
          name: container.Names,
          description: `${container.Image}`,
          icon: '▶️',
        }));
    },
  },
};
```

**Variadic Example:**

```typescript
{
  name: "rm",
  description: "Remove one or more containers",
  args: {
    name: "containers",
    isVariadic: true,
    generators: dockerGenerators.allDockerContainers,
  },
  options: [
    {
      name: ["-f", "--force"],
      description: "Force removal of running containers",
    },
    {
      name: ["-v", "--volumes"],
      description: "Remove volumes associated with container",
    },
  ],
}
```

### 3.4 Kubectl Spec Analysis

**File:** `src/kubectl.ts`
**Size:** ~3,500 lines
**Complexity:** Very High (Kubernetes resource management)

**Structure:**

- **Nesting depth:** 2-3 levels
- **70+ subcommands**
- **Shared argument library:** resourcesArg, runningPodsArg
- **Context-aware generators:** cluster, namespace, resource type

**Resource Type Generator:**

```typescript
const resourceTypeGenerator: Fig.Generator = {
  script: 'kubectl api-resources',
  postProcess: (output) => {
    const lines = output.split('\n').slice(1); // Skip header

    return lines.map((line) => {
      const parts = line.trim().split(/\s+/);
      const name = parts[0];
      const shortNames = parts[1] !== 'false' ? parts[1] : '';
      const apiGroup = parts[2];

      return {
        name,
        description: `API: ${apiGroup}${shortNames ? ` (${shortNames})` : ''}`,
        icon: '📋',
      };
    });
  },
  cache: {
    strategy: 'stale-while-revalidate',
    ttl: 1000 * 60 * 60, // 1 hour
  },
};
```

**Dynamic Resource Suggestions:**

```typescript
const resourceSuggestionsFromResourceType: Fig.Generator = {
  custom: async (tokens, executeShellCommand) => {
    // Find the resource type from previous tokens
    const resourceType = tokens[tokens.indexOf('get') + 1];
    if (!resourceType) return [];

    const namespace = tokens.includes('-n')
      ? tokens[tokens.indexOf('-n') + 1]
      : '';

    const namespaceFlag = namespace ? `-n ${namespace}` : '';
    const cmd = `kubectl get ${resourceType} ${namespaceFlag} -o name`;

    try {
      const output = await executeShellCommand(cmd);
      return output
        .split('\n')
        .filter(Boolean)
        .map((name) => ({
          name: name.split('/')[1] || name, // Strip resource type prefix
          description: resourceType,
        }));
    } catch {
      return [];
    }
  },
  trigger: (newToken, oldToken) => newToken !== oldToken,
};
```

**Shared Arguments Pattern:**

```typescript
const sharedArgs = {
  resourcesArg: {
    name: 'resource',
    generators: resourceTypeGenerator,
  },

  runningPodsArg: {
    name: 'pod',
    generators: {
      script: 'kubectl get pods -o name',
      postProcess: (output) => {
        return output
          .split('\n')
          .filter(Boolean)
          .map((name) => ({
            name: name.replace('pod/', ''),
            icon: '🎯',
          }));
      },
    },
  },
};
```

### 3.5 AWS S3 Spec Analysis

**File:** `src/aws/s3.ts`
**Size:** ~1,500 lines
**Complexity:** Moderate-High

**Structure:**

- **10 top-level subcommands** (no nesting)
- **4 reusable generators**
- **Heavy option duplication** (cp, mv, sync share encryption/metadata options)

**Generators:**

```typescript
const generators = {
  listFilesGenerator: {
    template: 'filepaths',
  },

  listBlobsGenerator: {
    script: 'ls -1ApL',
    postProcess: (output) => {
      return output
        .split('\n')
        .filter(Boolean)
        .map((name) => ({
          name: `fileb://${name}`,
          description: 'Binary file',
        }));
    },
  },

  listRemoteFilesGenerator: {
    trigger: (newToken, oldToken) => {
      return newToken.startsWith('s3://') && newToken !== oldToken;
    },
    getQueryTerm: (token) => {
      return token.split('/').pop() || '';
    },
    script: (tokens) => {
      const s3Path = tokens[tokens.length - 1];
      return `aws s3 ls ${s3Path}`;
    },
    postProcess: (output) => {
      return output
        .split('\n')
        .filter(Boolean)
        .map((line) => {
          const isPrefixLine = line.includes('PRE ');
          const name = isPrefixLine
            ? line.split('PRE ')[1].trim()
            : line.split(/\s+/).pop();

          return {
            name,
            description: isPrefixLine ? 'S3 prefix' : 'S3 object',
            icon: isPrefixLine ? '📁' : '📄',
          };
        });
    },
    cache: {
      strategy: 'stale-while-revalidate',
      ttl: 1000 * 30, // 30 seconds
    },
  },

  kmsKeyIdGenerator: {
    script: 'aws kms list-keys',
    postProcess: (output) => {
      const data = JSON.parse(output);
      return data.Keys.map((key) => ({
        name: key.KeyId,
        description: 'KMS Key',
      }));
    },
    cache: {
      strategy: 'stale-while-revalidate',
      ttl: 1000 * 30,
    },
  },
};
```

### 3.6 Cargo Spec Analysis

**File:** `src/cargo.ts`
**Size:** ~3,500 lines
**Complexity:** Very High (Rust ecosystem integration)

**Structure:**

- **40+ subcommands**
- **12+ custom generators**
- **Workspace-aware completions**
- **External API integration** (crates.io search)

**Key Features:**

- **Metadata-driven:** Parses `cargo metadata` JSON output
- **Context detection:** Distinguishes workspace vs local packages
- **Version search:** Special trigger for `@` to switch from crate to version search
- **Feature detection:** Parses Cargo.toml for available features

**Package Generator (workspace-aware):**

```typescript
const rootPackageOrLocal = () => {
  return `cargo metadata --format-version=1 | jq '.packages[] | select(.source == null)'`;
};

const packageGenerator: Fig.Generator = {
  script: rootPackageOrLocal(),
  postProcess: (output) => {
    const packages = output
      .split('\n')
      .filter(Boolean)
      .map((line) => JSON.parse(line));

    return packages.map((pkg) => ({
      name: pkg.name,
      description: `v${pkg.version} - ${pkg.description || ''}`,
      icon: '📦',
    }));
  },
};
```

**Crates.io Search Generator:**

```typescript
const searchGenerator: Fig.Generator = {
  custom: async (tokens, executeShellCommand) => {
    const searchTerm = tokens[tokens.length - 1] || '';
    const isVersionSearch = searchTerm.includes('@');

    if (isVersionSearch) {
      const [crateName] = searchTerm.split('@');
      const cmd = `curl -s 'https://crates.io/api/v1/crates/${crateName}'`;
      const output = await executeShellCommand(cmd);
      const data = JSON.parse(output);

      return data.versions.map((v) => ({
        name: `${crateName}@${v.num}`,
        description: v.created_at,
      }));
    } else {
      const cmd = `curl -s 'https://crates.io/api/v1/crates?q=${searchTerm}&per_page=20'`;
      const output = await executeShellCommand(cmd);
      const data = JSON.parse(output);

      return data.crates.map((c) => ({
        name: c.name,
        description: c.description,
        icon: '🦀',
      }));
    }
  },
  trigger: (newToken, oldToken) => newToken !== oldToken,
  cache: {
    strategy: 'stale-while-revalidate',
    ttl: 1000 * 60 * 60, // 1 hour
  },
};
```

**Features Generator (Cargo.toml parsing):**

```typescript
const featuresGenerator: Fig.Generator = {
  script: 'cat Cargo.toml',
  postProcess: (output) => {
    const featureSection = output.match(/\[features\]([\s\S]*?)(\[|$)/);
    if (!featureSection) return [];

    const features = featureSection[1]
      .split('\n')
      .map((line) => line.trim())
      .filter((line) => line && !line.startsWith('#'))
      .map((line) => line.split('=')[0].trim());

    return features.map((name) => ({
      name,
      description: 'Cargo feature',
      icon: '⚙️',
    }));
  },
};
```

**Target Generator (build targets):**

```typescript
const targetGenerator: Fig.Generator = {
  script: `cargo metadata --format-version=1 | jq '.packages[].targets[]'`,
  postProcess: (output, tokens) => {
    const targets = output
      .split('\n')
      .filter(Boolean)
      .map((line) => JSON.parse(line));

    // Filter by kind if specified (bin, lib, example, test, bench)
    const kindFilter = tokens.find((t) =>
      ['bin', 'lib', 'example', 'test', 'bench'].includes(t)
    );

    return targets
      .filter((t) => !kindFilter || t.kind.includes(kindFilter))
      .map((t) => ({
        name: t.name,
        description: `${t.kind.join(', ')} target`,
        icon: t.kind.includes('bin') ? '📦' : '📚',
      }));
  },
};
```

**Configuration Generator (key-value pairs):**

```typescript
const configGenerator: Fig.Generator = {
  custom: async (tokens) => {
    const lastToken = tokens[tokens.length - 1];

    if (lastToken.includes('.')) {
      // Value suggestions based on key
      const [section, key] = lastToken.split('.');

      if (section === 'cargo-new' && key === 'vcs') {
        return [
          { name: 'git', description: 'Git repository' },
          { name: 'hg', description: 'Mercurial repository' },
          { name: 'none', description: 'No version control' },
        ];
      }
    } else {
      // Key suggestions
      return [
        { name: 'cargo-new.vcs', description: 'VCS for new projects' },
        { name: 'build.jobs', description: 'Number of parallel jobs' },
        { name: 'term.color', description: 'Color output' },
        // ... etc
      ];
    }

    return [];
  },
};
```

### 3.7 AWS Main Spec (Lazy Loading Pattern)

**File:** `src/aws.ts`
**Size:** ~500 lines (lists hundreds of services)
**Complexity:** Low (orchestration layer)

**Structure:**

- **Hundreds of service subcommands**
- **Each uses `loadSpec` for lazy loading**
- **Shared generators** (awsProfile)
- **Minimal inline definitions**

**LoadSpec Pattern:**

```typescript
const completionSpec: Fig.Spec = {
  name: 'aws',
  description: 'AWS CLI',
  options: [
    {
      name: '--profile',
      description: 'Use a specific AWS profile',
      args: {
        name: 'profile',
        generators: awsProfileGenerator,
      },
    },
    {
      name: '--region',
      description: 'AWS region',
      args: {
        name: 'region',
        suggestions: [
          'us-east-1',
          'us-west-2',
          'eu-west-1',
          // ... all regions
        ],
      },
    },
  ],
  subcommands: [
    {
      name: 's3',
      description: 'Amazon Simple Storage Service',
      loadSpec: 'aws/s3', // Loads on-demand
    },
    {
      name: 'ec2',
      description: 'Amazon Elastic Compute Cloud',
      loadSpec: 'aws/ec2',
    },
    {
      name: 'lambda',
      description: 'AWS Lambda',
      loadSpec: 'aws/lambda',
    },
    // ... hundreds more services
  ],
};
```

**Profile Generator (shared):**

```typescript
const awsProfileGenerator: Fig.Generator = {
  script: 'cat ~/.aws/config',
  postProcess: (output) => {
    const profiles = output.match(/\[profile (.*?)\]/g) || [];
    return profiles.map((match) => {
      const name = match.replace('[profile ', '').replace(']', '');
      return {
        name,
        description: 'AWS profile',
        icon: '👤',
      };
    });
  },
  cache: {
    strategy: 'stale-while-revalidate',
    ttl: 1000 * 60 * 60, // 1 hour
    cacheByDirectory: true,
  },
};
```

**Benefits of this pattern:**

- Initial load is fast (only main spec parsed)
- Memory efficient (services loaded only when accessed)
- Easy to maintain (each service in separate file)
- Consistent structure across services

---

## 4. Generator Deep Dive

### 4.1 Generator Execution Flow

1. **Trigger evaluation:** Check if cache should be invalidated
2. **Cache check:** If cached and valid, return cached suggestions
3. **Execution:**
   - **Script-based:** Execute shell command
   - **Custom:** Call async function
   - **Template:** Use built-in generator
4. **Post-processing:** Transform output to `Suggestion[]`
5. **Filtering:** Apply `getQueryTerm` and filter strategy
6. **Cache update:** Store results with TTL

### 4.2 Generator Performance Patterns

**Good practices observed:**

1. **Cache aggressively for slow operations:**

   ```typescript
   {
     script: "curl https://api.npms.io/...",
     cache: { ttl: 1000 * 60 * 60 * 24 * 2 },  // 2 days
   }
   ```

2. **Use stale-while-revalidate for good UX:**

   ```typescript
   {
     cache: {
       strategy: "stale-while-revalidate",  // Show old data while fetching
     },
   }
   ```

3. **Trigger only when necessary:**

   ```typescript
   {
     trigger: (newToken, oldToken) => {
       return newToken.length > 3 && newToken !== oldToken;
     },
   }
   ```

4. **Parse efficiently:**

   ```typescript
   postProcess: (output) => {
     // JSON parsing when possible
     return output.split('\n').map((line) => JSON.parse(line));
   };
   ```

### 4.3 Generator Anti-Patterns

**Issues observed in specs:**

1. **No timeout specification:**
   - Some generators don't set `scriptTimeout`
   - Can hang UI on slow commands

2. **Expensive operations without caching:**
   - Some recursive directory searches
   - Should use caching with directory-based keys

3. **Complex awk/sed in scripts:**
   - Hard to maintain
   - Better to use simple commands + postProcess

4. **Error handling missing:**
   - Many postProcess functions don't handle empty/malformed output
   - Should use try/catch and return empty array

---

## 5. Parser Directives

Control how Fig parses command tokens.

### 5.1 Available Directives

```typescript
{
  parserDirectives: {
    // Options must come before arguments
    optionsMustPrecedeArguments: boolean,

    // Flags are NOT POSIX-compliant
    // (e.g., -work is ONE flag, not -w -o -r -k)
    flagsArePosixNoncompliant: boolean,

    // Expand aliases before offering completions
    alias: {
      [key: string]: string | string[],
    },
  },
}
```

### 5.2 Real Examples

**From Go spec:**

```typescript
{
  name: "go",
  parserDirectives: {
    flagsArePosixNoncompliant: true,  // -work is single flag
  },
}
```

**From Screen spec:**

```typescript
{
  name: "screen",
  parserDirectives: {
    optionsMustPrecedeArguments: true,
  },
}
```

**Alias expansion:**

```typescript
{
  name: "git",
  parserDirectives: {
    alias: {
      co: "checkout",
      ci: "commit",
      st: "status",
    },
  },
}
```

---

## 6. Build Process & TypeScript Constructs

### 6.1 Build Pipeline

1. **Input:** TypeScript files in `src/`
2. **Type checking:** `tsc --noEmit` validates types
3. **Compilation:** `@withfig/autocomplete-tools compile`
4. **Output:** JavaScript files in `build/`

**Build script from package.json:**

```json
{
  "scripts": {
    "build": "autocomplete-tools compile src",
    "test": "tsc --noEmit",
    "dev": "autocomplete-tools dev",
    "create-spec": "autocomplete-tools create-spec",
    "lint": "eslint src --ext .ts",
    "lint:fix": "eslint src --ext .ts --fix"
  }
}
```

### 6.2 TypeScript Constructs Used

**Imports:**

```typescript
// Type imports only (stripped at runtime)
import type { Fig } from '@withfig/autocomplete-types';

// Helper utilities (runtime dependencies)
import { filepaths, keyValue } from '@fig/autocomplete-generators';
import { semver } from 'semver';
import stripJsonComments from 'strip-json-comments';
```

**Type annotations:**

```typescript
const completionSpec: Fig.Spec = {
  name: 'example',
  // ...
};

const myGenerator: Fig.Generator = {
  script: 'ls',
  // ...
};
```

**Conditional logic (build-time vs runtime):**

Most specs are **declarative data**, not executable code. However, generator functions contain runtime JavaScript:

```typescript
{
  custom: async (tokens, executeShellCommand) => {
    // This JavaScript runs at runtime in Fig
    const output = await executeShellCommand("ls");
    return output.split("\n").map(name => ({ name }));
  },
}
```

### 6.3 Runtime Dependencies

**Minimal runtime dependencies observed:**

1. **@fig/autocomplete-generators:** Helper functions like `filepaths()`
2. **@fig/autocomplete-helpers:** Utility functions
3. **semver:** Version comparison
4. **strip-json-comments:** Parse JSON with comments
5. **yaml:** Parse YAML files

**Note:** These are used sparingly. Most specs are pure data.

### 6.4 Static Analysis Potential

**Can be statically extracted:**

- Subcommand names and descriptions
- Option names and descriptions
- Static argument suggestions
- Spec structure and nesting
- RequiresSeparator, isRepeatable, etc.

**Cannot be statically extracted:**

- Generator script outputs (requires execution)
- Dynamic script functions (requires JS evaluation)
- Custom generator logic (arbitrary async functions)
- PostProcess transformations (arbitrary JS)

**Implication for autocomplete-rs:**

You can build a **two-tier system:**

1. **Static tier:** Parse TypeScript AST to extract declarative structure
2. **Dynamic tier:** Execute JavaScript generators at runtime (or stub them out)

For initial implementation, focus on **static-only specs** or **template generators** (filepaths, folders, etc).

---

## 7. Spec Ecosystem Analysis

### 7.1 Repository Statistics

- **Total specs:** 500+ CLI tools
- **License:** MIT
- **Primary language:** TypeScript (100%)
- **Total commits:** 4,897+ (as of research date)
- **Stars:** 25.1k
- **Forks:** 5.5k
- **Contributors:** 400+ (many first-time open source contributors)

### 7.2 Spec Size Distribution

Based on analyzed samples:

| Size Range        | Examples       | Complexity       |
| ----------------- | -------------- | ---------------- |
| 50-200 lines      | cd, ls, hello  | Simple utilities |
| 200-500 lines     | Basic CLIs     | Low-moderate     |
| 500-1,500 lines   | git, aws/s3    | Moderate         |
| 1,500-2,500 lines | npm, docker    | High             |
| 2,500+ lines      | kubectl, cargo | Very high        |

**Largest observed:** kubectl (~3,500 lines), cargo (~3,500 lines)

**Median (estimated):** ~400-600 lines

### 7.3 Maintenance Status

**Activity indicators:**

- **Last verified commit:** Active development ongoing
- **Issue tracker:** Active on github.com/withfig/fig
- **Community:** Strong contributor base
- **Corporate backing:** Amazon (formerly Fig Inc.)

**Spec quality varies:**

- Popular tools (git, npm, docker) are well-maintained
- Niche tools may be outdated
- Some specs have TODOs indicating incomplete features

### 7.4 Spec Categories

Common CLI tool categories:

1. **Version control:** git, gh, gitlab
2. **Package managers:** npm, yarn, cargo, pip
3. **Containers:** docker, kubectl, helm
4. **Cloud providers:** aws, gcloud, azure
5. **Build tools:** make, cmake, gradle, maven
6. **Dev tools:** tsc, eslint, prettier
7. **System utilities:** ls, cd, find, grep

---

## 8. Design Implications for autocomplete-rs

### 8.1 Parser Architecture Recommendations

**Recommended approach:**

1. **Define Rust types mirroring Fig namespace:**

   ```rust
   struct Spec {
       name: String,
       description: Option<String>,
       subcommands: Vec<Subcommand>,
       options: Vec<Option>,
       args: Option<Args>,
       // ...
   }
   ```

2. **Start with static subset:**
   - Parse subcommands, options, args
   - Support static suggestions
   - Support template generators (filepaths, folders)
   - Ignore custom/script generators initially

3. **Add generator support incrementally:**
   - Phase 1: Template generators only
   - Phase 2: Script generators (execute shell commands)
   - Phase 3: PostProcess (consider embedded JS runtime or skip)
   - Phase 4: Custom generators (requires JS runtime like Deno/QuickJS)

### 8.2 Spec Loading Strategy

**Options:**

**Option A: TypeScript AST parsing (complex but complete)**

- Use `swc` or `tree-sitter-typescript` to parse .ts files
- Extract object literals into Rust structs
- Skip function bodies initially
- **Pros:** Works with existing specs as-is
- **Cons:** Complex, fragile, can't handle all TypeScript

**Option B: JSON intermediate format (recommended)**

- Fork `@withfig/autocomplete-tools` or create custom compiler
- Compile TypeScript specs to pure JSON (strip generators)
- Load JSON in autocomplete-rs
- **Pros:** Simple, robust, type-safe
- **Cons:** Loses dynamic generators (acceptable for v1)

**Option C: Hybrid approach**

- Parse TypeScript for static parts
- Mark generator fields as "external" with script strings
- Execute generators via spawned processes
- **Pros:** Flexible, can support subset of generators
- **Cons:** More complex than JSON approach

### 8.3 Recommended Phase 1 Scope

**Must-have:**

- Parse subcommand trees (name, description, nesting)
- Parse options (name, description, args, isRequired)
- Parse args (name, isVariadic, isOptional)
- Static suggestions
- Template: "filepaths" and "folders"
- Priority ranking

**Nice-to-have:**

- exclusiveOn / dependsOn
- parserDirectives (basic support)
- Hidden suggestions
- Icon support (for UI rendering)

**Defer to later:**

- Script generators
- Custom generators
- PostProcess functions
- Cache configuration
- Complex parserDirectives

### 8.4 Example Rust Type Definitions

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Spec {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default)]
    pub subcommands: Vec<Subcommand>,
    #[serde(default)]
    pub options: Vec<Option>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub args: Option<Args>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub load_spec: Option<String>,
    #[serde(default)]
    pub hidden: bool,
    #[serde(default)]
    pub is_dangerous: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Subcommand {
    #[serde(deserialize_with = "string_or_seq_string")]
    pub name: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default)]
    pub subcommands: Vec<Subcommand>,
    #[serde(default)]
    pub options: Vec<Option>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub args: Option<Args>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Option {
    #[serde(deserialize_with = "string_or_seq_string")]
    pub name: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub args: Option<Args>,
    #[serde(default)]
    pub is_required: bool,
    #[serde(default)]
    pub is_persistent: bool,
    #[serde(default)]
    pub exclusive_on: Vec<String>,
    #[serde(default)]
    pub depends_on: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Args {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default)]
    pub suggestions: Vec<Suggestion>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub template: Option<Template>,
    #[serde(default)]
    pub is_variadic: bool,
    #[serde(default)]
    pub is_optional: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Suggestion {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Template {
    Filepaths,
    Folders,
    History,
    Help,
}

// Helper deserializer for fields that can be string or array
fn string_or_seq_string<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::{self, Deserialize};

    struct StringOrVec;

    impl<'de> de::Visitor<'de> for StringOrVec {
        type Value = Vec<String>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("string or array of strings")
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(vec![value.to_string()])
        }

        fn visit_seq<A>(self, seq: A) -> Result<Self::Value, A::Error>
        where
            A: de::SeqAccess<'de>,
        {
            Deserialize::deserialize(de::value::SeqAccessDeserializer::new(seq))
        }
    }

    deserializer.deserialize_any(StringOrVec)
}
```

### 8.5 Spec Storage Approach

**Recommended:**

1. **Bundle pre-compiled JSON specs:**
   - Include popular CLIs (git, npm, docker, etc.) as embedded JSON
   - Use `include_str!()` macro or build-time code generation
   - Ship with autocomplete-rs binary

2. **Support user-provided specs:**
   - Allow loading from `~/.config/autocomplete-rs/specs/`
   - Watch directory for changes
   - Hot-reload specs without daemon restart

3. **Lazy loading for large spec trees:**
   - Implement `load_spec` support
   - Cache loaded specs in memory
   - Use LRU eviction if memory constrained

### 8.6 Generator Execution Strategy

**For Phase 1 (static-only):**

```rust
pub enum Generator {
    Template(TemplateType),
    External {
        script: String,
        // Mark as unimplemented for now
    },
}

impl Generator {
    pub fn generate(&self, context: &CompletionContext) -> Vec<Suggestion> {
        match self {
            Generator::Template(TemplateType::Filepaths) => {
                generate_filepath_suggestions(context)
            }
            Generator::Template(TemplateType::Folders) => {
                generate_folder_suggestions(context)
            }
            Generator::External { .. } => {
                // TODO: implement script execution
                vec![]
            }
        }
    }
}
```

**For Phase 2 (script generators):**

```rust
use tokio::process::Command;

impl Generator {
    pub async fn generate(&self, context: &CompletionContext) -> Vec<Suggestion> {
        match self {
            Generator::Script { script, post_process } => {
                let output = Command::new("sh")
                    .arg("-c")
                    .arg(script)
                    .output()
                    .await?;

                let stdout = String::from_utf8_lossy(&output.stdout);

                // Apply simple post-processing (line splitting)
                if post_process.is_none() {
                    stdout.lines()
                        .map(|line| Suggestion {
                            name: line.to_string(),
                            ..Default::default()
                        })
                        .collect()
                } else {
                    // TODO: complex post-processing requires JS runtime
                    vec![]
                }
            }
            // ...
        }
    }
}
```

### 8.7 Testing Strategy

1. **Unit tests for parser:**
   - Parse minimal spec
   - Parse nested subcommands
   - Parse options with args
   - Parse variadic args

2. **Integration tests with real specs:**
   - Bundle 5-10 representative specs (simple to complex)
   - Test parsing of each
   - Verify completion generation for known inputs

3. **Benchmark with large specs:**
   - kubectl, cargo, aws
   - Measure parse time
   - Measure completion generation time
   - Set performance targets (e.g., <10ms for completion)

### 8.8 Documentation Recommendations

Create these docs:

1. **`docs/fig-spec-format.md`** - Overview of Fig spec structure
2. **`docs/spec-conversion.md`** - How to convert TS specs to JSON
3. **`docs/generator-support.md`** - Which generator features are supported
4. **`docs/adding-specs.md`** - Guide for users to add their own specs

---

## 9. Key Takeaways

### 9.1 Spec Format Strengths

1. **Declarative:** Easy to author and maintain
2. **Type-safe:** TypeScript catches errors early
3. **Extensible:** Generator system handles any dynamic logic
4. **Community-driven:** 500+ specs, 400+ contributors
5. **Battle-tested:** Powers Amazon Q Developer CLI

### 9.2 Spec Format Weaknesses

1. **JavaScript-dependent:** Generators require JS runtime
2. **No formal schema:** Type definitions are authoritative, but spread across docs
3. **Inconsistent quality:** Some specs are outdated or incomplete
4. **Performance concerns:** Complex generators can be slow
5. **Limited validation:** Build process doesn't validate generator correctness

### 9.3 Compatibility Considerations

**If targeting Fig spec compatibility:**

- **Pros:** Access to 500+ existing specs
- **Cons:** Must support JavaScript generators (complex)

**If using Fig as inspiration only:**

- **Pros:** Design your own simpler format
- **Cons:** Users must write new specs or convert

**Recommended hybrid:**

- Parse Fig specs for static structure
- Provide conversion tool for common patterns
- Document differences clearly
- Support subset of generators (templates + simple scripts)

---

## 10. Appendix: Additional Resources

### 10.1 Official Documentation

- [Fig Autocomplete Docs](https://fig.gitbook.io/fig/autocomplete)
- [Fig Reference: Spec](https://fig.io/docs/reference/subcommand)
- [Fig Reference: Generator](https://fig.io/docs/reference/generator)
- [Fig Reference: Option](https://fig.io/docs/reference/option)
- [Fig Reference: Arg](https://fig.io/docs/reference/arg)
- [Fig Reference: Suggestion](https://fig.io/docs/reference/suggestion)
- [Fig Getting Started](https://fig.io/docs/getting-started)
- [Building Your First Spec](https://fig.io/docs/guides/building-first-spec)

### 10.2 GitHub Resources

- [withfig/autocomplete](https://github.com/withfig/autocomplete) - Main spec repository
- [withfig/autocomplete-tools](https://github.com/withfig/autocomplete-tools) - Build tools
- [withfig/fig](https://github.com/withfig/fig) - Issue tracker

### 10.3 NPM Packages

- [@withfig/autocomplete](https://www.npmjs.com/package/@withfig/autocomplete) - Compiled specs
- [@withfig/autocomplete-types](https://www.npmjs.com/package/@withfig/autocomplete-types) - TypeScript types
- [@withfig/autocomplete-tools](https://www.npmjs.com/package/@withfig/autocomplete-tools) - CLI tools

### 10.4 Example Spec Files Analyzed

Simple specs:

- [src/cd.ts](https://github.com/withfig/autocomplete/blob/master/src/cd.ts)
- [src/ls.ts](https://github.com/withfig/autocomplete/blob/master/src/ls.ts)

Moderate specs:

- [src/git.ts](https://github.com/withfig/autocomplete/blob/master/src/git.ts)
- [src/npm.ts](https://github.com/withfig/autocomplete/blob/master/src/npm.ts)
- [src/docker.ts](https://github.com/withfig/autocomplete/blob/master/src/docker.ts)
- [src/aws/s3.ts](https://github.com/withfig/autocomplete/blob/master/src/aws/s3.ts)

Complex specs:

- [src/kubectl.ts](https://github.com/withfig/autocomplete/blob/master/src/kubectl.ts)
- [src/cargo.ts](https://github.com/withfig/autocomplete/blob/master/src/cargo.ts)
- [src/aws.ts](https://github.com/withfig/autocomplete/blob/master/src/aws.ts)

---

## Sources

Research compiled from:

1. [GitHub - withfig/autocomplete: IDE-style autocomplete for your existing terminal & shell](https://github.com/withfig/autocomplete)
2. [Autocomplete | Fig Docs](https://fig.gitbook.io/fig/autocomplete)
3. [@withfig/autocomplete-types - npm](https://www.npmjs.com/package/@withfig/autocomplete-types)
4. [@withfig/autocomplete - npm](https://www.npmjs.com/package/@withfig/autocomplete)
5. [Docs | Generators](https://fig.io/docs/reference/generator)
6. [Docs | Subcommand](https://fig.io/docs/reference/subcommand)
7. [Docs | Option](https://fig.io/docs/reference/option)
8. [Docs | Argument](https://fig.io/docs/reference/arg)
9. [Docs | Suggestion](https://fig.io/docs/reference/suggestion)
10. [Docs | Getting Started](https://fig.io/docs/getting-started)
11. [Docs | Creating your First Completion Spec](https://fig.io/docs/getting-started/first-completion-spec)
12. [Autocomplete for Terminal Commands: A Deep Dive into Fig's Open-Source Engine](https://www.blog.brightcoding.dev/2025/09/10/autocomplete-for-terminal-commands-a-deep-dive-into-figs-open-source-engine/)
13. [autocomplete/src/git.ts at master · withfig/autocomplete](https://github.com/withfig/autocomplete/blob/master/src/git.ts)
14. [autocomplete/src/npm.ts at master · withfig/autocomplete](https://github.com/withfig/autocomplete/blob/master/src/npm.ts)
15. [autocomplete/src/docker.ts at master · withfig/autocomplete](https://github.com/withfig/autocomplete/blob/master/src/docker.ts)
16. [autocomplete/package.json at master · withfig/autocomplete](https://github.com/withfig/autocomplete/blob/master/package.json)
17. [autocomplete/screen.ts at master · withfig/autocomplete](https://github.com/withfig/autocomplete/blob/master/src/screen.ts)
18. [autocomplete/src/cargo.ts at master · withfig/autocomplete](https://github.com/withfig/autocomplete/blob/master/src/cargo.ts)
