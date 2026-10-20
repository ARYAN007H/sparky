import { readFileSync } from "node:fs";
import { join } from "node:path";

const USER_TEXT = "Describe the attached image briefly.";
const REPLY = "The image is attached.";
const FILE_NAME = "typed-roundtrip.png";
const IMAGE_BYTES = Buffer.from(
  "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNk+A8AAQUBAScY42YAAAAASUVORK5CYII=",
  "base64",
);

function unwrap(value) {
  for (let depth = 0; depth < 6 && value && !value.message; depth += 1) {
    value = value.result ?? value.data ?? value.value;
  }
  return value;
}

export default async function typedImageAttachmentRoundtrip({ page, mock, log, logDir }) {
  const rpcCalls = [];
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
  mock.set("llmStreamScript", [{ text: REPLY }, { finish: "stop" }]);

  await page.goto(new URL("/#/chat", page.url()).toString());
  const composer = page.getByRole("textbox", { name: "Message input" });
  await composer.waitFor({ state: "visible", timeout: 30_000 });
  let threadId = await page.evaluate(
    () => window.__OPENHUMAN_STORE__?.getState?.().thread?.selectedThreadId ?? null,
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

  await page.locator('input[type="file"]').setInputFiles({
    name: FILE_NAME,
    mimeType: "image/png",
    buffer: IMAGE_BYTES,
  });
  await page.getByText(FILE_NAME, { exact: true }).waitFor({ state: "visible" });
  await composer.fill(USER_TEXT);
  await page.getByTestId("send-message-button").click();
  await page.getByText(REPLY, { exact: false }).waitFor({ timeout: 90_000 });

  const append = rpcCalls.find((call) => call.method === "openhuman.threads_message_append");
  if (!append) throw new Error("composer did not persist the user message through the core");
  const part = append.params?.attachments?.[0];
  const dataUri = part?.media?.url;
  if (
    part?.kind !== "image" ||
    part?.media?.contentType !== "image/png" ||
    part?.media?.fileName !== FILE_NAME ||
    !dataUri?.startsWith("data:image/png;name=")
  ) {
    throw new Error(`core append lacked the typed PNG payload: ${JSON.stringify(part)}`);
  }

  const stagedMessage = unwrap(
    await page.evaluate(async ({ rpcUrl: url, token, threadId: id }) => {
      const response = await fetch(url, {
        method: "POST",
        headers: { authorization: token, "content-type": "application/json" },
        body: JSON.stringify({
          jsonrpc: "2.0",
          id: "typed-image-attachment-roundtrip-read",
          method: "openhuman.threads_messages_list",
          params: { thread_id: id },
        }),
      });
      return response.json();
    }, { rpcUrl, token: authorization, threadId }),
  );
  const persisted = stagedMessage?.messages?.find(
    (message) => message.sender === "user" && message.content.includes(FILE_NAME),
  );
  if (!persisted) throw new Error("typed image was not persisted in the thread message list");
  if (/\[(?:IMAGE|FILE):/i.test(persisted.content))
    throw new Error("modern typed upload leaked a legacy [IMAGE:]/[FILE:] marker");
  const encoded = persisted.content.match(/\[ATTACHMENT:([^\]]+)\]/)?.[1];
  if (!encoded) throw new Error("persisted message has no durable attachment reference");
  const reference = JSON.parse(decodeURIComponent(encoded));
  if (
    reference.name !== FILE_NAME ||
    reference.mime !== "image/png" ||
    reference.size_bytes !== IMAGE_BYTES.length ||
    !reference.path.startsWith("uploads/")
  ) {
    throw new Error(`staged reference lost image metadata: ${JSON.stringify(reference)}`);
  }
  const stagedBytes = readFileSync(join(logDir, "workspace", reference.path));
  if (!stagedBytes.equals(IMAGE_BYTES))
    throw new Error("staged image bytes differ from the browser upload");
  log(`core staged ${reference.mime} ${reference.name} (${stagedBytes.length} bytes) at ${reference.path}`);

  const chatRequest = rpcCalls.find((call) => call.method === "openhuman.channel_web_chat");
  if (!chatRequest || chatRequest.params?.message !== persisted.content)
    throw new Error("chat turn did not reuse the staged durable reference as its message");
  if (/\[(?:IMAGE|FILE):/i.test(chatRequest.params.message))
    throw new Error("chat turn sent a legacy attachment marker");
  if (chatRequest.params?.attachments?.length)
    throw new Error("ordinary attachment was staged twice as a chat part and durable reference");

  const inferenceRequest = mock.requests().find((request) =>
    /(?:^|\/)chat\/completions(?:\?|$)/.test(request.url),
  );
  if (!inferenceRequest)
    throw new Error("mock backend did not receive the model request for the image turn");
  const inferenceBody = JSON.parse(inferenceRequest.body);
  const userMessages = inferenceBody.messages?.filter((message) => message.role === "user") ?? [];
  const userText = JSON.stringify(userMessages);
  if (/\[(?:ATTACHMENT|IMAGE|FILE):/i.test(userText))
    throw new Error("legacy or durable staging markers reached the model's user text");
  if (!userText.includes(IMAGE_BYTES.toString("base64")))
    throw new Error("model request did not receive the uploaded image bytes as typed media");
  log("mock model request contained typed image bytes and clean user text");

  await page.reload();
  await composer.waitFor({ state: "visible", timeout: 30_000 });
  await page.evaluate((id) => window.__OPENHUMAN_STORE__?.dispatch({
    type: "thread/setSelectedThread",
    payload: id,
  }), threadId);
  await page.getByText(REPLY, { exact: false }).waitFor({ timeout: 30_000 });
  const restored = await page.evaluate((id) => {
    const messages = window.__OPENHUMAN_STORE__?.getState?.().thread?.messagesByThreadId?.[id] ?? [];
    return messages.find((message) => message.sender === "user" && message.content.includes("typed-roundtrip.png"))?.content;
  }, threadId);
  if (restored !== persisted.content)
    throw new Error("reloaded conversation did not restore the durable image reference");
  log("typed image attachment survived core staging, chat submission and reload");
}
