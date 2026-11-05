import { test } from "node:test";
import assert from "node:assert/strict";
import { readFile, mkdir } from "node:fs/promises";
import { spawnSync } from "node:child_process";
import { resolve, dirname } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const project = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const root = resolve(project, "../..");
const tools = resolve(
  root,
  "packages/wasm-crypto/node_modules/@bytecodealliance",
);
const output = resolve(root, ".local/xiaobai-component-tests");
await mkdir(output, { recursive: true });
const result = spawnSync(
  process.execPath,
  [
    resolve(tools, "jco/dist/jco.js"),
    "transpile",
    resolve(project, "dist/adapter.component.wasm"),
    "-o",
    output,
    "--instantiation",
    "async",
  ],
  { encoding: "utf8" },
);
assert.equal(result.status, 0, result.stderr);
const { instantiate } = await import(
  pathToFileURL(resolve(output, "adapter.component.js")).href
);
const { WASIShim } = await import(
  pathToFileURL(resolve(tools, "preview2-shim/dist/common/instantiation.js"))
    .href
);

const base = "https://music.90svip.cn/";
const homeFinal = "https://music.90svip.cn/home-final";
const rowUrl = "row";
const rowRequest = `${base}${rowUrl}`;
const finalUrl = "https://user:secret@edge.example:9443/final.mp3?token=final";
const largePayload = 14 * 1024 * 1024;
const media = (() => {
  const data = new Uint8Array(6890 * 1044);
  for (let offset = 0; offset < data.length; offset += 1044)
    data.set([255, 251, 224, 0], offset);
  return data;
})();

function request(extra = {}) {
  return {
    track: {
      reference: { source: "netease", kind: "track", id: "123" },
      title: "Test track",
      aliases: [],
      artist: "Test artist",
      artists: [],
      album: "",
      albumReference: null,
      durationMs: 180000,
      cover: {
        original: "https://cover.example:9443/cover.jpg",
        small: "https://cover.example:9443/cover.jpg",
        normal: "https://cover.example:9443/cover.jpg",
        large: "https://cover.example:9443/cover.jpg",
        exlarge: "https://cover.example:9443/cover.jpg",
      },
      associations: [],
    },
    preferredQuality: "320k",
    allowDowngrade: false,
    ...extra,
  };
}

async function component({
  postPadding = 200 * 1024,
  homePadding = 200 * 1024,
  mediaSize = media.length,
  httpFailure = null,
} = {}) {
  const requests = [];
  const diagnostics = [];
  let homeCalls = 0;
  const range = (value) => {
    const header = value.headers.find(
      (item) => item.name.toLowerCase() === "range",
    );
    const [start, end] = header.value
      .slice("bytes=".length)
      .split("-")
      .map(Number);
    return [start, Math.min(end, mediaSize - 1)];
  };
  const transport = {
    httpRequest(value) {
      requests.push(value);
      assert.equal(value.timeoutMs, 0, value.url);
      assert.equal(value.followRedirects, true, value.url);
      if (httpFailure) throw httpFailure;
      if (value.method === "POST") {
        assert.ok(
          value.headers.some((header) => header.name === "Content-Type"),
        );
        const body = JSON.stringify({
          code: 200,
          data: [{ songid: "123", url: "row" }],
        });
        return {
          url: "https://music.90svip.cn/resolved/",
          status: 200,
          headers: [{ name: "content-type", value: "application/json" }],
          body: new TextEncoder().encode(
            body.padEnd(body.length + postPadding, " "),
          ),
        };
      }
      if (value.url === base) {
        homeCalls += 1;
        const body =
          '<script>document.cookie="mk_visit=visit;path=/";</script>';
        return {
          url: homeFinal,
          status: 200,
          headers: [
            { name: "content-type", value: "text/html" },
            {
              name: "set-cookie",
              value: "session=anonymous; Path=/; Secure; HttpOnly",
            },
          ],
          body: new TextEncoder().encode(
            body.padEnd(body.length + homePadding, " "),
          ),
        };
      }
      if (value.url === homeFinal) {
        homeCalls += 1;
        assert.ok(value.headers.some((header) => header.name === "Cookie"));
        return {
          url: homeFinal,
          status: 200,
          headers: [{ name: "content-type", value: "text/html" }],
          body: new TextEncoder().encode("<html>ready</html>"),
        };
      }
      if (value.url === rowRequest || value.url === finalUrl) {
        const [start, end] = range(value);
        const actualEnd = Math.min(end, media.length - 1);
        let body =
          start < media.length
            ? media.slice(start, actualEnd + 1)
            : new Uint8Array();
        if (body.length < end - start + 1) {
          const extended = new Uint8Array(end - start + 1);
          extended.set(body);
          for (
            let offset = body.length;
            offset < extended.length;
            offset += 1044
          )
            extended.set([255, 251, 224, 0], offset);
          body = extended;
        }
        return {
          url: finalUrl,
          status: 206,
          headers: [
            {
              name: "Content-Range",
              value: `bytes ${start}-${end}/${mediaSize}`,
            },
            { name: "ETag", value: '"fixture"' },
          ],
          body,
        };
      }
      return {
        url: value.url,
        status: 404,
        headers: [],
        body: new Uint8Array(),
      };
    },
  };
  const instance = await instantiate(
    async (path) => WebAssembly.compile(await readFile(resolve(output, path))),
    {
      ...new WASIShim().getImportObject(),
      "nons:host/transport": transport,
      "nons:plugin/host": {
        call(operation, args) {
          assert.equal(operation, "diagnostic.report");
          diagnostics.push(JSON.parse(args));
          return "null";
        },
      },
    },
  );
  instance.initialize();
  return {
    instance,
    requests,
    diagnostics,
    homeCalls: () => homeCalls,
    call(method, value) {
      return JSON.parse(instance.call(method, JSON.stringify(value)));
    },
  };
}

test("descriptor uses contract 2 without legacy network fields", async () => {
  const fixture = await component();
  try {
    const descriptor = fixture.call("music.resource-descriptor", null);
    assert.equal(descriptor.status, "ok");
    assert.equal(descriptor.data.contractVersion, 2);
    assert.equal(Object.hasOwn(descriptor.data, "coverHosts"), false);
  } finally {
    fixture.instance.shutdown();
  }
});

test("resolve follows Host final URLs and accepts responses and arguments above old limits", async () => {
  const fixture = await component();
  try {
    const input = {
      request: request(),
      padding: "x".repeat(200 * 1024),
    };
    assert.ok(JSON.stringify(input).length > 128 * 1024);
    const response = fixture.call("music.resolve-resource", input);
    assert.equal(response.status, "ok", JSON.stringify(response));
    assert.equal(response.data.url, finalUrl);
    assert.equal(response.data.metadata.actualQuality, "320k");
    assert.equal(fixture.homeCalls(), 2);

    const post = fixture.requests.find((value) => value.method === "POST");
    assert.ok(post);
    assert.equal(
      new TextDecoder().decode(post.body),
      "input=123&filter=id&type=netease&page=1",
    );
    const rows = fixture.requests.filter((value) => value.url === rowRequest);
    assert.equal(rows.length, 1);
    const probes = fixture.requests.filter(
      (value) => value.url === rowRequest || value.url === finalUrl,
    );
    assert.ok(probes.length > 1);
    assert.equal(probes[0].url, rowRequest);
    assert.ok(probes.slice(1).every((value) => value.url === finalUrl));
    assert.equal(
      probes
        .slice(1)
        .some((value) =>
          value.headers.some((header) => header.name === "Cookie"),
        ),
      false,
    );
    assert.equal(fixture.diagnostics.length, 0);
  } finally {
    fixture.instance.shutdown();
  }
});

test("generic responses above 12 MiB are accepted", async () => {
  const fixture = await component({ postPadding: largePayload });
  try {
    const response = fixture.call("music.resolve-resource", {
      request: request(),
    });
    assert.equal(response.status, "ok", JSON.stringify(response));
    assert.equal(response.data.url, finalUrl);
  } finally {
    fixture.instance.shutdown();
  }
});

test("generic arguments above 12 MiB are accepted", async () => {
  const fixture = await component();
  try {
    const input = { request: request(), padding: "x".repeat(largePayload) };
    assert.ok(Buffer.byteLength(JSON.stringify(input)) > 12 * 1024 * 1024);
    const response = fixture.call("music.resolve-resource", input);
    assert.equal(response.status, "ok", JSON.stringify(response));
  } finally {
    fixture.instance.shutdown();
  }
});

test("reported media size above 512 MiB is accepted", async () => {
  const fixture = await component({ mediaSize: 600 * 1024 * 1024 });
  try {
    const input = request();
    input.track.durationMs = 15_742_000;
    const response = fixture.call("music.resolve-resource", {
      request: input,
    });
    assert.equal(response.status, "ok", JSON.stringify(response));
    assert.equal(response.data.url, finalUrl);
  } finally {
    fixture.instance.shutdown();
  }
});

test("independent component instances accept more than the old concurrency budget", async () => {
  const fixtures = await Promise.all(
    Array.from({ length: 40 }, () => component()),
  );
  try {
    const responses = await Promise.all(
      fixtures.map((fixture) =>
        Promise.resolve(
          fixture.call("music.resolve-resource", { request: request() }),
        ),
      ),
    );
    for (const response of responses) {
      assert.equal(response.status, "ok", JSON.stringify(response));
      assert.equal(response.data.url, finalUrl);
    }
  } finally {
    await Promise.all(fixtures.map((fixture) => fixture.instance.shutdown()));
  }
});

test("Host timeout errors keep their code and diagnostic class", async () => {
  const fixture = await component({ httpFailure: "timeout" });
  try {
    const response = fixture.call("music.resolve-resource", {
      request: request(),
    });
    assert.equal(response.status, "error");
    assert.equal(response.data.code, "timeout");
    assert.deepEqual(fixture.diagnostics.at(-1), {
      stage: "music.call",
      class: "network",
      code: "timeout",
    });
  } finally {
    fixture.instance.shutdown();
  }
});
