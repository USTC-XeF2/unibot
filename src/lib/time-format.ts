const LOCALE = "zh-CN";

function isSameDay(date: Date, now = new Date()): boolean {
  return (
    date.getFullYear() === now.getFullYear() &&
    date.getMonth() === now.getMonth() &&
    date.getDate() === now.getDate()
  );
}

function withUnixSeconds(
  ts: number,
  formatter: (date: Date) => string,
): string {
  if (!ts) {
    return "";
  }

  return formatter(new Date(ts * 1000));
}

export function formatConversationPreviewTime(ts: number): string {
  return withUnixSeconds(ts, (date) => {
    if (isSameDay(date)) {
      return date.toLocaleTimeString(LOCALE, {
        hour: "2-digit",
        minute: "2-digit",
      });
    }

    return date.toLocaleDateString(LOCALE, {
      month: "2-digit",
      day: "2-digit",
    });
  });
}

export function formatMessageTimestamp(ts: number): string {
  return withUnixSeconds(ts, (date) => {
    if (isSameDay(date)) {
      return date.toLocaleTimeString(LOCALE, {
        hour: "2-digit",
        minute: "2-digit",
        second: "2-digit",
      });
    }

    const datePart = date.toLocaleDateString(LOCALE, {
      month: "2-digit",
      day: "2-digit",
    });
    const timePart = date.toLocaleTimeString(LOCALE, {
      hour: "2-digit",
      minute: "2-digit",
      second: "2-digit",
    });

    return `${datePart} ${timePart}`;
  });
}

export function formatMonthDayTime(ts: number): string {
  return withUnixSeconds(ts, (date) =>
    date.toLocaleString(LOCALE, {
      month: "2-digit",
      day: "2-digit",
      hour: "2-digit",
      minute: "2-digit",
    }),
  );
}
