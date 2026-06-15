import { listen } from "@tauri-apps/api/event";
import { render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, type Mock, vi } from "vitest";
import { ChatEventBusProvider } from "../chat-event-bus-provider";

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(),
}));

describe("ChatEventBusProvider", () => {
  const mockedListen = listen as Mock;

  beforeEach(() => {
    mockedListen.mockReset();
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
});
