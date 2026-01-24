#!/usr/bin/env node
/**
 * Fake Google Docs API Server
 *
 * Mocks the Google Drive API v3 endpoints needed for listing and exporting Google Docs.
 * Used in e2e tests to avoid real Google API calls.
 *
 * Endpoints:
 * - GET /drive/v3/files - List documents (requires Bearer token)
 * - GET /drive/v3/files/:id/export - Export document as markdown (requires Bearer token)
 */

import { createServer } from "http";

const PORT = 8788;
// Valid tokens from fake-oauth-server.js
const EXPECTED_ACCESS_TOKEN = "fake_access_token_12345"; // Initial token
const EXPECTED_REFRESHED_TOKEN = "fake_access_token_12345_refreshed"; // Refreshed token

const FAKE_DOCS = [
  {
    id: "fake_doc_id_001",
    name: "Sample Google Doc for Testing",
    modifiedTime: "2024-01-25T12:00:00.000Z",
  },
  {
    id: "fake_doc_id_002",
    name: "Another Test Document",
    modifiedTime: "2024-01-24T10:30:00.000Z",
  },
];

const FAKE_MARKDOWN_CONTENT = `# Sample Google Doc

This is fake content from a mocked Google Doc.

## Chapter 1

"Hello," she said.

"World," he replied.
`;

function handleRequest(req, res) {
  const { pathname } = new URL(req.url, `http://localhost:${PORT}`);

  console.log(`[FAKE-GOOGLE-API] ${req.method} ${pathname}`);

  // Verify Authorization header for all endpoints
  const authHeader = req.headers.authorization;
  console.log(`[FAKE-GOOGLE-API] 🔍 Received Authorization header: "${authHeader}"`);

  if (!authHeader) {
    console.log("[FAKE-GOOGLE-API] ❌ Missing Authorization header");
    res.writeHead(401, { "Content-Type": "application/json" });
    res.end(JSON.stringify({ error: { message: "Unauthorized: Missing Authorization header", code: 401 } }));
    return;
  }

  // Accept both initial and refreshed tokens
  const validTokens = [
    `Bearer ${EXPECTED_ACCESS_TOKEN}`,
    `Bearer ${EXPECTED_REFRESHED_TOKEN}`
  ];

  if (!validTokens.includes(authHeader)) {
    console.log(`[FAKE-GOOGLE-API] ❌ Invalid token. Expected one of: ${validTokens.join(", ")}, Got: "${authHeader}"`);
    res.writeHead(401, { "Content-Type": "application/json" });
    res.end(JSON.stringify({ error: { message: "Unauthorized: Invalid token", code: 401 } }));
    return;
  }

  console.log(`[FAKE-GOOGLE-API] ✅ Authorization valid - token: ${authHeader}`);

  // List documents endpoint
  if (pathname === "/drive/v3/files" || pathname.startsWith("/drive/v3/files?")) {
    console.log("[FAKE-GOOGLE-API] Returning fake docs list");
    res.writeHead(200, { "Content-Type": "application/json" });
    res.end(JSON.stringify({ files: FAKE_DOCS }));
    return;
  }

  // Export document as markdown
  if (pathname.match(/^\/drive\/v3\/files\/[^/]+\/export/)) {
    console.log("[FAKE-GOOGLE-API] Returning fake markdown content");
    res.writeHead(200, { "Content-Type": "text/plain" });
    res.end(FAKE_MARKDOWN_CONTENT);
    return;
  }

  res.writeHead(404);
  res.end("Not Found");
}

export function startFakeGoogleDocsApi() {
  return new Promise((resolve) => {
    const server = createServer(handleRequest);
    server.listen(PORT, "127.0.0.1", () => {
      console.log(`[FAKE-GOOGLE-API] Listening on http://127.0.0.1:${PORT}`);
      resolve(server);
    });
  });
}

export function stopFakeGoogleDocsApi(server) {
  return new Promise((resolve) => {
    if (!server) {
      resolve();
      return;
    }
    server.close(() => {
      console.log("[FAKE-GOOGLE-API] Server stopped");
      resolve();
    });
  });
}

// Allow running standalone for manual testing
if (import.meta.url === `file://${process.argv[1]}`) {
  startFakeGoogleDocsApi().then(() => {
    console.log("[FAKE-GOOGLE-API] Running in standalone mode. Press Ctrl+C to exit.");
  });
}
