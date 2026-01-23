import { vi } from "vitest";
import "@testing-library/jest-dom/vitest";

// Mock @tauri-apps/api/core (invoke)
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
  Channel: vi.fn(),
}));

// Mock @tauri-apps/api/event
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(() => Promise.resolve(() => {})),
  once: vi.fn(() => Promise.resolve(() => {})),
  emit: vi.fn(),
}));

// Mock bindings commands and events
vi.mock("../bindings", () => ({
  commands: {
    loadSession: vi.fn(() =>
      Promise.resolve({ status: "ok" as const, data: null })
    ),
    saveSession: vi.fn(() =>
      Promise.resolve({ status: "ok" as const, data: null })
    ),
    scanBooks: vi.fn(() =>
      Promise.resolve({ status: "ok" as const, data: [] })
    ),
    loadCoverImage: vi.fn(() =>
      Promise.resolve({
        status: "ok" as const,
        data: { data_url: "data:image/png;base64,abc123", width: 100, height: 150 },
      })
    ),
    loadBookMetadata: vi.fn(() =>
      Promise.resolve({ status: "ok" as const, data: {} })
    ),
    saveBookMetadata: vi.fn(() =>
      Promise.resolve({ status: "ok" as const, data: null })
    ),
    selectFolder: vi.fn(() =>
      Promise.resolve({ status: "ok" as const, data: null })
    ),
    gitGetStatus: vi.fn(() =>
      Promise.resolve({
        status: "ok" as const,
        data: { status: "Uninitialized" },
      })
    ),
    // GitHub auth commands
    githubCheckAuth: vi.fn(() =>
      Promise.resolve({ status: "ok" as const, data: false })
    ),
    githubDeviceFlowStart: vi.fn(() =>
      Promise.resolve({
        status: "ok" as const,
        data: {
          deviceCode: "test-device-code",
          userCode: "TEST-CODE",
          verificationUri: "https://github.com/login/device",
          expiresIn: 900,
        },
      })
    ),
    githubDeviceFlowPoll: vi.fn(() =>
      Promise.resolve({ status: "ok" as const, data: "test-token" })
    ),
    githubStoreToken: vi.fn(() =>
      Promise.resolve({ status: "ok" as const, data: null })
    ),
    githubLogout: vi.fn(() =>
      Promise.resolve({ status: "ok" as const, data: null })
    ),
    openBrowser: vi.fn(() =>
      Promise.resolve({ status: "ok" as const, data: null })
    ),
    gitCheckInitialized: vi.fn(() =>
      Promise.resolve({ status: "ok" as const, data: false })
    ),
    gitCloneRepository: vi.fn(() =>
      Promise.resolve({ status: "ok" as const, data: null })
    ),
    gitSync: vi.fn(() =>
      Promise.resolve({ status: "ok" as const, data: "Synced successfully" })
    ),
    gitSave: vi.fn(() =>
      Promise.resolve({ status: "ok" as const, data: "Saved successfully" })
    ),
    // Google auth commands
    googleCheckAuth: vi.fn(() =>
      Promise.resolve({ status: "ok" as const, data: false })
    ),
    googleAuthStart: vi.fn(() =>
      Promise.resolve({ status: "ok" as const, data: null })
    ),
    googleAuthCancel: vi.fn(() =>
      Promise.resolve({ status: "ok" as const, data: null })
    ),
    googleLogout: vi.fn(() =>
      Promise.resolve({ status: "ok" as const, data: null })
    ),
    googleListDocs: vi.fn(() =>
      Promise.resolve({ status: "ok" as const, data: [] })
    ),
    googleLinkDoc: vi.fn(() =>
      Promise.resolve({ status: "ok" as const, data: null })
    ),
    googleSyncDoc: vi.fn(() =>
      Promise.resolve({ status: "ok" as const, data: "Synced" })
    ),
    googleUnlinkDoc: vi.fn(() =>
      Promise.resolve({ status: "ok" as const, data: null })
    ),
  },
  events: {
    processingUpdateEvent: {
      listen: vi.fn(() => Promise.resolve(() => {})),
      once: vi.fn(() => Promise.resolve(() => {})),
      emit: vi.fn(),
    },
    gitOperationProgressEvent: {
      listen: vi.fn(() => Promise.resolve(() => {})),
      once: vi.fn(() => Promise.resolve(() => {})),
      emit: vi.fn(),
    },
    googleDocsProgressEvent: {
      listen: vi.fn(() => Promise.resolve(() => {})),
      once: vi.fn(() => Promise.resolve(() => {})),
      emit: vi.fn(),
    },
    bookListChangedEvent: {
      listen: vi.fn(() => Promise.resolve(() => {})),
      once: vi.fn(() => Promise.resolve(() => {})),
      emit: vi.fn(),
    },
    updateProgressEvent: {
      listen: vi.fn(() => Promise.resolve(() => {})),
      once: vi.fn(() => Promise.resolve(() => {})),
      emit: vi.fn(),
    },
  },
}));

// Stable mock functions for react-i18next (must be stable to avoid infinite effect loops)
const mockT = (key: string) => key;
const mockChangeLanguage = vi.fn();
const mockI18n = {
  changeLanguage: mockChangeLanguage,
  language: "en",
};

// Mock react-i18next with stable references
vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: mockT,
    i18n: mockI18n,
  }),
  Trans: ({ children }: { children: React.ReactNode }) => children,
  initReactI18next: {
    type: "3rdParty",
    init: vi.fn(),
  },
}));
