/**
 * Flow spec factory — the primary reusable spec set. For every {@link UserFlow}
 * in `@ppt/sitemap`'s `allFlows` whose `apps` includes the app under test, it
 * registers one test that drives the flow through {@link FlowRunner}
 * (specs-as-data: the catalog is the spec, the runner is the engine).
 *
 * Backend gating: flows that need a live backend (see {@link flowNeedsBackend})
 * are tagged `@requires-backend` in the title and `test.skip()` cleanly unless
 * `E2E_WITH_BACKEND=1` — so a code-only run is honest, never green-by-stub.
 *
 * Prerequisites: a flow that lists `prerequisites` (e.g. login before logout)
 * runs those prior flows first through the same runner, so the composed state
 * is established without duplicating steps. Prerequisite flows that themselves
 * need a backend pull the whole composite under the backend gate.
 *
 * Tags: `flow.tags` (smoke / critical / …) are surfaced both in the test title
 * and as test annotations so reporters and `--grep` can target them.
 */

import { allFlows, getFlow, type UserFlow } from '@ppt/sitemap';
import { test } from '../fixtures';
import { backendEnabled, FlowRunner, flowNeedsBackend } from '../flow/FlowRunner';

export interface RegisterFlowOptions {
  readonly app: string;
}

/** Flow apps overlap frontend + backend ids; match the frontend app only. */
function flowTargetsApp(flow: UserFlow, app: string): boolean {
  return (flow.apps as readonly string[]).includes(app);
}

/** Resolve the transitive prerequisite chain (depth-first, de-duplicated). */
function resolvePrerequisites(flow: UserFlow, seen = new Set<string>()): UserFlow[] {
  const chain: UserFlow[] = [];
  for (const prereqId of flow.prerequisites ?? []) {
    if (seen.has(prereqId)) {
      continue;
    }
    seen.add(prereqId);
    const prereq = getFlow(prereqId);
    if (!prereq) {
      continue;
    }
    chain.push(...resolvePrerequisites(prereq, seen), prereq);
  }
  return chain;
}

/** Does the flow OR any of its prerequisites require a backend? */
function compositeNeedsBackend(flow: UserFlow): boolean {
  if (flowNeedsBackend(flow)) {
    return true;
  }
  return resolvePrerequisites(flow).some(flowNeedsBackend);
}

/** Build the `@<tag>` suffix for a flow title from its tags. */
function tagSuffix(flow: UserFlow, requiresBackend: boolean): string {
  const tags = new Set(flow.tags ?? []);
  const parts: string[] = [];
  if (tags.has('critical')) {
    parts.push('@critical');
  }
  if (tags.has('smoke')) {
    parts.push('@smoke');
  }
  if (requiresBackend) {
    parts.push('@requires-backend');
  }
  return parts.length > 0 ? ` ${parts.join(' ')}` : '';
}

/**
 * Register a flow-driven test per matching flow. Call inside a `test.describe`.
 */
export function registerFlowSpecs({ app }: RegisterFlowOptions): void {
  const flows = allFlows.filter((flow) => flowTargetsApp(flow, app));

  if (flows.length === 0) {
    test.skip(`no flows target '${app}'`, () => {});
    return;
  }

  for (const flow of flows) {
    const requiresBackend = compositeNeedsBackend(flow);
    const title = `${flow.id} · ${flow.name}${tagSuffix(flow, requiresBackend)}`;

    test(title, async ({ page }, testInfo) => {
      for (const tag of flow.tags ?? []) {
        testInfo.annotations.push({ type: 'flow-tag', description: tag });
      }
      testInfo.annotations.push({ type: 'flow-id', description: flow.id });

      if (requiresBackend && !backendEnabled()) {
        test.skip(true, `${flow.id} requires a backend — set E2E_WITH_BACKEND=1 to run`);
        return;
      }

      // Establish prerequisite state by running prior flows first.
      for (const prereq of resolvePrerequisites(flow)) {
        await new FlowRunner(page, prereq, testInfo).run();
      }

      await new FlowRunner(page, flow, testInfo).run();
    });
  }
}
