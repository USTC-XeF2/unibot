import { QueryClientProvider } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { render, screen, waitFor } from "@testing-library/react";
import type { ReactNode } from "react";
import { MemoryRouter, Route, Routes } from "react-router";
import { beforeEach, describe, expect, it, type Mock, vi } from "vitest";
import { createTestQueryClient } from "@/test/utils";
import GroupAlbumsWindow from "../group-albums-window";

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(),
}));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

function renderWithRouter(initialEntries: string[]) {
  const queryClient = createTestQueryClient();

  function Wrapper({ children: _children }: { children: ReactNode }) {
    return (
      <QueryClientProvider client={queryClient}>
        <MemoryRouter initialEntries={initialEntries}>
          <Routes>
            <Route path="/group-albums" element={<GroupAlbumsWindow />} />
          </Routes>
        </MemoryRouter>
      </QueryClientProvider>
    );
  }

  return render(<div />, { wrapper: Wrapper });
}

describe("GroupAlbumsWindow", () => {
  const mockedInvoke = invoke as Mock;
  const mockedListen = listen as Mock;

  beforeEach(() => {
    mockedInvoke.mockReset();
    mockedListen.mockReset();
    mockedListen.mockResolvedValue(vi.fn());
    mockedInvoke.mockImplementation((cmd) => {
      if (cmd === "list_group_albums" || cmd === "list_group_photos") {
        return Promise.resolve([]);
      }
      return Promise.reject(new Error(`unexpected invoke: ${String(cmd)}`));
    });
  });

  it("renders error message for missing params", () => {
    renderWithRouter(["/group-albums"]);
    expect(
      screen.getByText(/缺少有效的 userId 或 groupId/),
    ).toBeInTheDocument();
  });

  it("renders error message for invalid group id", () => {
    renderWithRouter(["/group-albums?userId=123&groupId=group?123"]);
    expect(
      screen.getByText(/缺少有效的 userId 或 groupId/),
    ).toBeInTheDocument();
  });

  it("renders browser for valid params", async () => {
    renderWithRouter(["/group-albums?userId=123&groupId=albums_1"]);

    expect(
      screen.queryByText(/缺少有效的 userId 或 groupId/),
    ).not.toBeInTheDocument();
    await waitFor(() => {
      expect(mockedListen).toHaveBeenCalled();
    });
  });
});
