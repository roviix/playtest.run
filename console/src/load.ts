// 取数的三种状态：在取、取到了、没取到。四个页面共用一套，省得每页各写一遍。

import { useCallback, useEffect, useState } from "preact/hooks";

import { ApiError } from "./api";

export type Loaded<T> = {
  data?: T;
  error?: ApiError;
  loading: boolean;
  reload: () => void;
};

export function useLoad<T>(fetcher: () => Promise<T>, deps: unknown[]): Loaded<T> {
  const [state, setState] = useState<{ data?: T; error?: ApiError; loading: boolean }>({
    loading: true,
  });
  const [attempt, setAttempt] = useState(0);
  const reload = useCallback(() => setAttempt((n) => n + 1), []);

  useEffect(() => {
    let alive = true;
    setState({ loading: true });
    fetcher().then(
      (data) => {
        if (alive) setState({ data, loading: false });
      },
      (error: unknown) => {
        if (!alive) return;
        setState({
          loading: false,
          error:
            error instanceof ApiError
              ? error
              : new ApiError(0, "internal", String((error as Error)?.message ?? error)),
        });
      },
    );
    return () => {
      alive = false;
    };
  }, [...deps, attempt]);

  return { ...state, reload };
}
