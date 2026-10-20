import { readdirSync, readFileSync, statSync } from "node:fs";
import { join } from "node:path";

const FIRST_REPLY = "The first turn is complete.";
const QUEUED_PROMPT = "Describe this queued image.";
const QUEUED_REPLY = "The queued image is attached.";
const FILE_NAME = "queued-roundtrip.png";
const IMAGE_BYTES = Buffer.from(
  "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNk+A8AAQUBAScY42YAAAAASUVORK5CYII=",
  "base64",
);

function unwrap(value) {
  for (let depth = 0; depth < 6 && value && typeof value === "object"; depth += 1) {
    if (Object.hasOwn(value, "error")) {
      throw new Error(`RPC returned an error: ${JSON.stringify(value.error)}`);
    }
    const wrapper = ["result", "data", "value"].find((key) =>
      Object.hasOwn(value, key),
    );
    if (!wrapper) break;
    value = value[wrapper];
  }
  return value;
}

function findStoredBytes(root, expected) {
  for (const name of readdirSync(root)) {
    const path = join(root, name);
    const stat = statSync(path);
    if (stat.isDirectory()) {
      const found = findStoredBytes(path, expected);
      if (found) return found;
    } else if (stat.isFile() && stat.size === expected.length) {
      if (readFileSync(path).equals(expected)) return expected;
    }
  }
  return null;
}

function hasSourceAuthority(value) {
  if (Array.isArray(value)) return value.some(hasSourceAuthority);
  if (!value || typeof value !== "object") return false;
  return Object.entries(value).some(([key, child]) =>
    (["path", "url", "localCachePath"].includes(key) &&
      typeof child === "string" &&
      child.length > 0) ||
      hasSourceAuthority(child),
  );
}

async function waitUntil(predicate, label, timeoutMs = 10_000) {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    const value = predicate();
    if (value) return value;
    await new Promise((resolve) => setTimeout(resolve, 50));
  }
  throw new Error(`timed out waiting for ${label}`);
}

export default async function queuedImageAttachmentRoundtrip({
  page,
  mock,
  log,
  logDir,
}) {
  const actionDir = process.env.OPENHUMAN_ACTION_DIR;
  if (!actionDir) throw new Error("OPENHUMAN_ACTION_DIR must isolate staged uploads");
  const rpcCalls = [];
  const rpcResponses = [];
  let rpcUrl;
  let authorization;
  page.on("request", async (request) => {
    if (!request.url().endsWith("/rpc")) return;
    rpcUrl = request.url();
    authorization = (await request.allHeaders()).authorization;
    try {
      const body = JSON.parse(request.postData() ?? "{}");
      rpcCalls.push({ method: body.method, params: body.params });
    } catch {}
  });
  page.on("response", async (response) => {
    if (!response.url().endsWith("/rpc")) return;
    try {
      const request = response.request();
      const body = JSON.parse(request.postData() ?? "{}");
      const responseBody = await response.json();
      try {
        rpcResponses.push({
          method: body.method,
          params: body.params,
          result: unwrap(responseBody),
        });
      } catch (error) {
        rpcResponses.push({
          method: body.method,
          params: body.params,
          error: String(error),
        });
      }
    } catch (error) {
      rpcResponses.push({ error: String(error) });
    }
  });

  // The mock captures a script when each model request starts. Hold the first
  // request before its first token, then change the script for the queued turn.
  mock.set("llmStreamScript", [
    { text: FIRST_REPLY, delayMs: 60_000 },
    { finish: "stop" },
  ]);
  await page.goto(new URL("/#/chat", page.url()).toString());
  const composer = page.getByRole("textbox", { name: "Message input" });
  await composer.waitFor({ state: "visible", timeout: 30_000 });
  let threadId = await page.evaluate(
    () =>
      window.__OPENHUMAN_STORE__?.getState?.().thread?.selectedThreadId ?? null,
  );
  if (!threadId) {
    await page.getByRole("button", { name: /new conversation/i }).click();
    threadId = await page
      .waitForFunction(
        () => window.__OPENHUMAN_STORE__?.getState?.().thread?.selectedThreadId,
        null,
        { timeout: 15_000 },
      )
      .then((handle) => handle.jsonValue());
  }
  if (!threadId) throw new Error("new chat thread was not selected");

  await composer.fill("Hold this turn while I queue an image.");
  await page.waitForTimeout(250);
  const firstSend = page.getByTestId("send-message-button");
  if (!(await firstSend.count())) {
    const value = await composer.inputValue().catch(() => "<not an input>");
    await page.screenshot({ path: join(logDir, "queued-first-send-missing.png") });
    throw new Error(`first send button missing after fill (draft=${JSON.stringify(value)})`);
  }
  await firstSend.click();
  await waitUntil(
    () =>
      mock
        .requests()
        .some((request) => /(?:^|\/)chat\/completions(?:\?|$)/.test(request.url)),
    "the first model request",
    30_000,
  );
  await page.waitForFunction(
    (id) =>
      window.__OPENHUMAN_STORE__?.getState?.().thread?.activeThreadIds?.[id] ===
      true,
    threadId,
    { timeout: 10_000 },
  );

  // The queued turn will use this fast script; the already-running request has
  // retained its delayed script.
  mock.set("llmStreamScript", [
    { text: QUEUED_REPLY },
    { finish: "stop" },
  ]);
  await page.locator('input[type="file"]').setInputFiles({
    name: FILE_NAME,
    mimeType: "image/png",
    buffer: IMAGE_BYTES,
  });
  await page
    .getByText(FILE_NAME, { exact: true })
    .waitFor({ state: "visible" });
  await composer.fill(QUEUED_PROMPT);
  await page.getByTestId("send-message-button").click();
  await page.getByTestId("queued-followups").waitFor({
    state: "visible",
    timeout: 15_000,
  });
  await page
    .getByTestId("queued-followups")
    .getByText(QUEUED_PROMPT, { exact: false })
    .waitFor({ state: "visible" });

  const queuedAckResponse = await waitUntil(
    () =>
      rpcResponses.find(
        (call) =>
          call.method === "openhuman.channel_web_chat" &&
          call.params?.message === QUEUED_PROMPT,
      ),
    "the queued channel_web_chat acknowledgement",
  );
  if (queuedAckResponse.error) throw new Error(queuedAckResponse.error);
  const ack = queuedAckResponse.result;
  if (ack?.queued !== true || !ack.request_id) {
    throw new Error(
      `queued channel_web_chat ACK is malformed: ${JSON.stringify(ack)}`,
    );
  }
  const pendingHandle = await page.waitForFunction(
    ({ id, requestId, attachments }) => {
      const pending = Object.values(
        window.__OPENHUMAN_STORE__?.getState?.().queue?.pendingFollowupsByThread?.[
          id
        ] ?? {},
      ).find((entry) => entry.requestId === requestId);
      return (
        pending &&
        JSON.stringify(pending.attachments) === JSON.stringify(attachments)
      );
    },
    {
      id: threadId,
      requestId: ack.request_id,
      attachments: ack.queued_attachments,
    },
    { timeout: 10_000 },
  );
  const pending = await pendingHandle.jsonValue();
  if (!pending?.requestId || pending.message?.content !== QUEUED_PROMPT) {
    throw new Error("accepted queued image did not remain as a matching pending follow-up");
  }
  if (JSON.stringify(ack.queued_attachments) !== JSON.stringify(pending.attachments)) {
    throw new Error(`queued ACK attachments differ from UI state: ${JSON.stringify(ack)}`);
  }
  if (
    pending.attachments?.length !== 1 ||
    pending.attachments[0]?.kind !== "image" ||
    !pending.attachments[0]?.media?.opaqueId
  ) {
    throw new Error(
      `queued ACK did not retain the core's opaque image ref: ${JSON.stringify(pending)}`,
    );
  }
  if (
    /data:[^\s\"]+|base64,/i.test(JSON.stringify(pending.attachments)) ||
    hasSourceAuthority(pending.attachments)
  ) {
    throw new Error("queued UI state retained upload bytes or a source path");
  }

  const earlyRows = await page.evaluate(
    async ({ url, token, id }) => {
      const response = await fetch(url, {
        method: "POST",
        headers: { authorization: token, "content-type": "application/json" },
        body: JSON.stringify({
          jsonrpc: "2.0",
          id: "queued-image-before-flush",
          method: "openhuman.threads_messages_list",
          params: { thread_id: id },
        }),
      });
      const body = await response.json();
      if (!response.ok) throw new Error(`thread list failed with HTTP ${response.status}`);
      if (body.error) throw new Error(`thread list RPC failed: ${JSON.stringify(body.error)}`);
      const messages = body.result?.messages ?? body.messages;
      if (!Array.isArray(messages)) {
        throw new Error(`thread list response has no messages: ${JSON.stringify(body)}`);
      }
      return messages;
    },
    { url: rpcUrl, token: authorization, id: threadId },
  );
  if (earlyRows.some((row) => row.content === QUEUED_PROMPT)) {
    throw new Error("queued image row was persisted before the active assistant turn finished");
  }
  const earlyUiRows =
    (await page.evaluate(
      (id) =>
        window.__OPENHUMAN_STORE__?.getState?.().thread?.messagesByThreadId?.[
          id
        ] ?? [],
      threadId,
    )) ?? [];
  if (earlyUiRows.some((row) => row.content === QUEUED_PROMPT)) {
    throw new Error("queued image appeared as a persisted transcript row before flush");
  }
  await page.getByTestId("stop-generation-button").waitFor({
    state: "visible",
    timeout: 5_000,
  });
  if (await page.getByText(FIRST_REPLY, { exact: false }).isVisible().catch(() => false)) {
    throw new Error("held active turn completed before the queued-row timing assertion");
  }
  log("queued image ACK retained only an opaque ref; no transcript row existed before the active turn completed");

  await page.getByText(FIRST_REPLY, { exact: false }).waitFor({ timeout: 60_000 });
  await page.getByText(QUEUED_REPLY, { exact: false }).waitFor({ timeout: 90_000 });

  const rows = await page.evaluate(
    async ({ url, token, id }) => {
      const response = await fetch(url, {
        method: "POST",
        headers: { authorization: token, "content-type": "application/json" },
        body: JSON.stringify({
          jsonrpc: "2.0",
          id: "queued-image-after-flush",
          method: "openhuman.threads_messages_list",
          params: { thread_id: id },
        }),
      });
      const body = await response.json();
      if (!response.ok) throw new Error(`thread list failed with HTTP ${response.status}`);
      if (body.error) throw new Error(`thread list RPC failed: ${JSON.stringify(body.error)}`);
      const messages = body.result?.messages ?? body.messages;
      if (!Array.isArray(messages)) {
        throw new Error(`thread list response has no messages: ${JSON.stringify(body)}`);
      }
      return messages;
    },
    { url: rpcUrl, token: authorization, id: threadId },
  );
  const userRows = rows.filter(
    (row) => row.sender === "user" && row.content === QUEUED_PROMPT,
  );
  if (userRows.length !== 1) {
    throw new Error(`queued image produced ${userRows.length} persisted user rows, expected one`);
  }
  const persisted = userRows[0];
  if (persisted.id !== `user:${pending.requestId}`) {
    throw new Error(
      `queued row ID ${persisted.id} does not match request-key ownership`,
    );
  }
  if (/\[(?:ATTACHMENT|IMAGE|FILE|VOICE):/i.test(persisted.content)) {
    throw new Error("queued typed image leaked a legacy marker into persisted content");
  }
  const reference = persisted.attachments?.[0];
  if (
    reference?.kind !== "image" ||
    reference.media?.contentType !== "image/png" ||
    reference.media?.fileName !== FILE_NAME ||
    reference.media?.sizeBytes !== IMAGE_BYTES.length ||
    !reference.media?.opaqueId ||
    reference.media?.path ||
    reference.media?.url
  ) {
    throw new Error(`queued image did not persist as an opaque typed ref: ${JSON.stringify(reference)}`);
  }
  if (!findStoredBytes(actionDir, IMAGE_BYTES)) {
    throw new Error("queued model image bytes differ from the browser upload");
  }

  const imageRequest = mock
    .requests()
    .filter((request) => /(?:^|\/)chat\/completions(?:\?|$)/.test(request.url))
    .at(-1);
  if (!imageRequest) throw new Error("queued turn did not reach the mock model");
  const input = JSON.stringify(JSON.parse(imageRequest.body).messages ?? []);
  if (/\[(?:ATTACHMENT|IMAGE|FILE|VOICE):/i.test(input)) {
    throw new Error("queued model input contains a legacy marker");
  }
  if (!input.includes(IMAGE_BYTES.toString("base64"))) {
    throw new Error("queued model input did not contain the actual PNG bytes");
  }

  await page.reload();
  await composer.waitFor({ state: "visible", timeout: 30_000 });
  await page.evaluate(
    (id) =>
      window.__OPENHUMAN_STORE__?.dispatch({
        type: "thread/setSelectedThread",
        payload: id,
      }),
    threadId,
  );
  await page.getByText(QUEUED_REPLY, { exact: false }).waitFor({ timeout: 30_000 });
  const reloaded = await page.evaluate(
    ({ id, rowId }) => {
      const messages =
        window.__OPENHUMAN_STORE__?.getState?.().thread?.messagesByThreadId?.[
          id
        ] ?? [];
      return messages.find((message) => message.sender === "user" && message.id === rowId);
    },
    { id: threadId, rowId: persisted.id },
  );
  if (
    reloaded?.content !== QUEUED_PROMPT ||
    JSON.stringify(reloaded?.attachments) !== JSON.stringify(persisted.attachments)
  ) {
    throw new Error("queued opaque image ref or clean text did not survive thread reload");
  }
  log(`queued image persisted once as ${persisted.id}; model received ${IMAGE_BYTES.length} typed PNG bytes`);
}
