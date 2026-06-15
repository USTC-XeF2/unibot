import { describe, expect, it } from "vitest";
import { isValidGroupId, isValidUserId } from "../common";

describe("isValidUserId", () => {
  it("accepts non-empty strings", () => {
    expect(isValidUserId("12345")).toBe(true);
    expect(isValidUserId(" 12345 ")).toBe(true);
  });

  it("rejects null, undefined, and empty strings", () => {
    expect(isValidUserId(null)).toBe(false);
    expect(isValidUserId(undefined)).toBe(false);
    expect(isValidUserId("")).toBe(false);
    expect(isValidUserId("   ")).toBe(false);
    expect(isValidUserId(12345 as unknown as string)).toBe(false);
  });
});

describe("isValidGroupId", () => {
  it("accepts alphanumeric, dash, and underscore", () => {
    expect(isValidGroupId("group_123-ABC")).toBe(true);
    expect(isValidGroupId("123")).toBe(true);
  });

  it("rejects empty or invalid values", () => {
    expect(isValidGroupId("")).toBe(false);
    expect(isValidGroupId(null)).toBe(false);
    expect(isValidGroupId(undefined)).toBe(false);
    expect(isValidGroupId("group/123")).toBe(false);
    expect(isValidGroupId("group 123")).toBe(false);
    expect(isValidGroupId("group?123")).toBe(false);
    expect(isValidGroupId("group&123")).toBe(false);
    expect(isValidGroupId("group.123")).toBe(false);
  });
});
