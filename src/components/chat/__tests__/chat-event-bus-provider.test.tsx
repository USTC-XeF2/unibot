import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, type Mock, vi } from "vitest";
import { ChatEventBusProvider } from "../chat-event-bus-provider";

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(),
}));

vi.mock("@tauri-apps/api/window", () => ({
  getCurrentWindow: vi.fn(),
}));

vi.mock("@/lib/query", () => ({
  handleQueryInvalidation: vi.fn(),
}));

describe("ChatEventBusProvider", () => {
  const mockedListen = listen as Mock;
  const mockedGetCurrentWindow = getCurrentWindow as Mock;

  beforeEach(() => {
    mockedListen.mockReset();
    mockedGetCurrentWindow.mockReset();
  });

  it("renders children when listener registration succeeds", async () => {
    mockedListen.mockResolvedValue(vi.fn());

    render(
      <ChatEventBusProvider userId="12345" windowLabel="chat-12345">
        <div data-testid="child">child content</div>
      </ChatEventBusProvider>,
    );

    expect(screen.getByTestId("child")).toBeInTheDocument();
  });

  it("shows an error banner when listener registration fails", async () => {
    mockedListen.mockRejectedValue(new Error("Tauri event bridge unavailable"));

    render(
      <ChatEventBusProvider userId="12345" windowLabel="chat-12345">
        <div data-testid="child">child content</div>
      </ChatEventBusProvider>,
    );

    await waitFor(() => {
      expect(screen.getByText(/实时连接断开/)).toBeInTheDocument();
    });
  });

  it("does not render children wrapper when userId is empty", () => {
    render(
      <ChatEventBusProvider userId="" windowLabel="chat-">
        <div data-testid="child">child content</div>
      </ChatEventBusProvider>,
    );

    expect(screen.getByTestId("child")).toBeInTheDocument();
    expect(mockedListen).not.toHaveBeenCalled();
  });

  it("closes the standalone window when its target user is removed from the group", async () => {
    const close = vi.fn().mockResolvedValue(undefined);
    mockedGetCurrentWindow.mockReturnValue({ close });
    let received: ((event: { payload: unknown }) => void) | undefined;
    mockedListen.mockImplementation(async (_event, handler) => {
      received = handler;
      return vi.fn();
    });

    render(
      <ChatEventBusProvider
        userId="10002"
        windowLabel="group-files-10002-20001"
      >
        <div data-testid="child">child content</div>
      </ChatEventBusProvider>,
    );

    await waitFor(() => expect(received).toBeDefined());
    received?.({
      payload: {
        kind: "group_member_left",
        group_id: "20001",
        operator_user_id: "10001",
        target_user_id: "10002",
        time: 100,
      },
    });

    await waitFor(() => expect(close).toHaveBeenCalledTimes(1));
  });

  it("keeps the window open when the member-left event targets the group of another window", async () => {
    const close = vi.fn().mockResolvedValue(undefined);
    mockedGetCurrentWindow.mockReturnValue({ close });
    let received: ((event: { payload: unknown }) => void) | undefined;
    mockedListen.mockImplementation(async (_event, handler) => {
      received = handler;
      return vi.fn();
    });

    render(
      <ChatEventBusProvider
        userId="10002"
        windowLabel="group-files-10002-20001"
      >
        <div data-testid="child">child content</div>
      </ChatEventBusProvider>,
    );

    await waitFor(() => expect(received).toBeDefined());
    received?.({
      payload: {
        kind: "group_member_left",
        group_id: "39999",
        operator_user_id: "10001",
        target_user_id: "10002",
        time: 100,
      },
    });

    await new Promise((resolve) => setTimeout(resolve, 0));
    expect(close).not.toHaveBeenCalled();
  });
});
