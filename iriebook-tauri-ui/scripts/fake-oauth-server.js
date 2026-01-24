#!/usr/bin/env node
/**
 * Fake OAuth Server for E2E Testing
 *
 * Simulates Google's OAuth endpoints locally for fast, deterministic testing.
 * No real Google API calls - perfect for automated tests!
 *
 * Endpoints:
 * - GET /o/oauth2/v2/auth - Returns auto-redirect HTML (simulates instant user consent)
 * - POST /token - Returns fake tokens (handles both authorization_code and refresh_token)
 */

import { createServer } from "http";
import { parse as parseUrl } from "url";

const PORT = 8787;
const FAKE_AUTH_CODE = "fake_auth_code_12345";
const FAKE_ACCESS_TOKEN = "fake_access_token_12345";
const FAKE_REFRESH_TOKEN = "fake_refresh_token_12345";

// Configurable delay (in ms) before auto-redirecting
// Set via OAUTH_DELAY_MS env var (default: 2000ms for visible UI testing)
const REDIRECT_DELAY_MS = parseInt(process.env.OAUTH_DELAY_MS || "2000", 10);

/**
 * Parse request body from POST request
 * @param {IncomingMessage} req - HTTP request
 * @returns {Promise<string>} - Request body
 */
function parseBody(req) {
  return new Promise((resolve, reject) => {
    let body = "";
    req.on("data", (chunk) => {
      body += chunk.toString();
    });
    req.on("end", () => resolve(body));
    req.on("error", reject);
  });
}

/**
 * Parse form-encoded data (application/x-www-form-urlencoded)
 * @param {string} body - Form-encoded string
 * @returns {Object} - Parsed form data
 */
function parseFormData(body) {
  const params = new URLSearchParams(body);
  const result = {};
  for (const [key, value] of params.entries()) {
    result[key] = value;
  }
  return result;
}

/**
 * Handle OAuth authorization endpoint
 * Returns HTML with auto-redirect to callback URL
 * @param {IncomingMessage} req
 * @param {ServerResponse} res
 */
function handleAuthEndpoint(req, res) {
  const { query } = parseUrl(req.url, true);
  const redirectUri = query.redirect_uri;

  if (!redirectUri) {
    res.writeHead(400, { "Content-Type": "text/plain" });
    res.end("Missing redirect_uri parameter");
    return;
  }

  console.log(`[FAKE-OAUTH] Auth request received, will redirect to: ${redirectUri} after ${REDIRECT_DELAY_MS}ms`);

  // Auto-redirect HTML with configurable delay
  const callbackUrl = `${redirectUri}?code=${FAKE_AUTH_CODE}`;
  const html = `
<!DOCTYPE html>
<html>
  <head>
    <title>Fake OAuth - Redirecting...</title>
    <style>
      body {
        font-family: system-ui, sans-serif;
        max-width: 600px;
        margin: 50px auto;
        padding: 20px;
        text-align: center;
      }
      .spinner {
        border: 4px solid #f3f3f3;
        border-top: 4px solid #3498db;
        border-radius: 50%;
        width: 40px;
        height: 40px;
        animation: spin 1s linear infinite;
        margin: 20px auto;
      }
      @keyframes spin {
        0% { transform: rotate(0deg); }
        100% { transform: rotate(360deg); }
      }
      #countdown { font-size: 2em; color: #3498db; }
    </style>
  </head>
  <body>
    <h1>🔐 Fake OAuth Server</h1>
    <div class="spinner"></div>
    <p>Simulating user sign-in for e2e testing...</p>
    <p>Redirecting in <span id="countdown">${Math.ceil(REDIRECT_DELAY_MS / 1000)}</span>s</p>
    <script>
      let timeLeft = ${REDIRECT_DELAY_MS};
      const countdownEl = document.getElementById('countdown');

      const interval = setInterval(() => {
        timeLeft -= 1000;
        if (timeLeft <= 0) {
          clearInterval(interval);
          countdownEl.textContent = '0';
        } else {
          countdownEl.textContent = Math.ceil(timeLeft / 1000);
        }
      }, 1000);

      // Auto-redirect to callback URL with auth code after delay
      setTimeout(() => {
        window.location.href = "${callbackUrl}";
      }, ${REDIRECT_DELAY_MS});
    </script>
  </body>
</html>`;

  res.writeHead(200, { "Content-Type": "text/html" });
  res.end(html);
}

/**
 * Handle token endpoint (both authorization_code and refresh_token grants)
 * @param {IncomingMessage} req
 * @param {ServerResponse} res
 */
async function handleTokenEndpoint(req, res) {
  const body = await parseBody(req);
  const params = parseFormData(body);

  console.log(`[FAKE-OAUTH] Token request received:`, params);

  const grantType = params.grant_type;

  if (grantType === "authorization_code") {
    // Exchange authorization code for tokens
    const tokenResponse = {
      access_token: FAKE_ACCESS_TOKEN,
      expires_in: 3600,
      refresh_token: FAKE_REFRESH_TOKEN,
      token_type: "Bearer",
      scope: "https://www.googleapis.com/auth/documents.readonly https://www.googleapis.com/auth/drive.readonly",
    };

    console.log(`[FAKE-OAUTH] Returning authorization_code token response`);
    res.writeHead(200, { "Content-Type": "application/json" });
    res.end(JSON.stringify(tokenResponse));
  } else if (grantType === "refresh_token") {
    // Refresh access token
    const tokenResponse = {
      access_token: FAKE_ACCESS_TOKEN + "_refreshed",
      expires_in: 3600,
      token_type: "Bearer",
      scope: "https://www.googleapis.com/auth/documents.readonly https://www.googleapis.com/auth/drive.readonly",
    };

    console.log(`[FAKE-OAUTH] Returning refresh_token response`);
    res.writeHead(200, { "Content-Type": "application/json" });
    res.end(JSON.stringify(tokenResponse));
  } else {
    console.log(`[FAKE-OAUTH] Unknown grant_type: ${grantType}`);
    res.writeHead(400, { "Content-Type": "application/json" });
    res.end(JSON.stringify({ error: "unsupported_grant_type" }));
  }
}

/**
 * Main request handler
 * @param {IncomingMessage} req
 * @param {ServerResponse} res
 */
async function handleRequest(req, res) {
  const { pathname } = parseUrl(req.url);

  console.log(`[FAKE-OAUTH] ${req.method} ${pathname}`);

  if (req.method === "GET" && pathname === "/o/oauth2/v2/auth") {
    handleAuthEndpoint(req, res);
  } else if (req.method === "POST" && pathname === "/token") {
    await handleTokenEndpoint(req, res);
  } else {
    res.writeHead(404, { "Content-Type": "text/plain" });
    res.end("Not Found");
  }
}

/**
 * Start the fake OAuth server
 * @returns {Promise<Server>} - HTTP server instance
 */
export function startFakeOAuthServer() {
  return new Promise((resolve, reject) => {
    const server = createServer(handleRequest);

    server.on("error", (err) => {
      console.error(`[FAKE-OAUTH] Server error: ${err.message}`);
      reject(err);
    });

    server.listen(PORT, "127.0.0.1", () => {
      console.log(`[FAKE-OAUTH] Server listening on http://127.0.0.1:${PORT}`);
      resolve(server);
    });
  });
}

/**
 * Stop the fake OAuth server
 * @param {Server} server - Server instance to stop
 * @returns {Promise<void>}
 */
export function stopFakeOAuthServer(server) {
  return new Promise((resolve) => {
    if (!server) {
      resolve();
      return;
    }

    server.close(() => {
      console.log("[FAKE-OAUTH] Server stopped");
      resolve();
    });
  });
}

// Run standalone if executed directly
if (import.meta.url === `file://${process.argv[1]}`) {
  console.log("=== Starting Fake OAuth Server (standalone mode) ===");
  startFakeOAuthServer().catch((err) => {
    console.error("Failed to start server:", err);
    process.exit(1);
  });
}
