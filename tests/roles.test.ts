import { test } from "vitest";
import assert from "node:assert/strict";
import { roles, rolesFor } from "../src/lib/roles.ts";

test("Claude Code implements and Codex reviews, with no role left unassigned", () => {
  assert.deepEqual(
    roles.map((item) => item.role),
    ["refinador", "implementador", "revisor", "tester"],
  );
  assert.deepEqual(
    rolesFor("claude").map((item) => item.role),
    ["refinador", "implementador"],
  );
  assert.deepEqual(
    rolesFor("codex").map((item) => item.role),
    ["revisor", "tester"],
  );
});

test("implementation and review never share a runtime", () => {
  const runtime = (role: string) => roles.find((item) => item.role === role)?.runtime;
  assert.notEqual(runtime("implementador"), runtime("revisor"));
  assert.notEqual(runtime("implementador"), runtime("tester"));
});
