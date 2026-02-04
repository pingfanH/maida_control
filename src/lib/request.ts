import axios from 'axios';

const envBase = import.meta.env.VITE_API_BASE_URL as string | undefined;
const baseURL =
  envBase ??
  (typeof window !== 'undefined'
    ? window.location.origin
    : 'http://127.0.0.1:9855');

const api = axios.create({
  baseURL, // API 基础地址
});

const syncSessionParams = () => {
  if (typeof window === "undefined") {
    return null;
  }
  const params = new URLSearchParams(window.location.search);
  const keys = ["user_id", "open_game_id", "session_id"];
  let updated = false;
  for (const key of keys) {
    const fromQuery = params.get(key);
    if (fromQuery) {
      window.localStorage.setItem(key, fromQuery);
      updated = true;
    }
  }
  if (updated) {
    return {
      user_id: params.get("user_id"),
      open_game_id: params.get("open_game_id"),
      session_id: params.get("session_id"),
    };
  }
  return {
    user_id: window.localStorage.getItem("user_id"),
    open_game_id: window.localStorage.getItem("open_game_id"),
    session_id: window.localStorage.getItem("session_id"),
  };
};

api.interceptors.request.use((config) => {
  const session = syncSessionParams();
  // if (session?.user_id) {
  //   if (!config.params) {
  //     config.params = { user_id: session.user_id };
  //   } else if (config.params instanceof URLSearchParams) {
  //     if (!config.params.has("user_id")) {
  //       config.params.set("user_id", session.user_id);
  //     }
  //   } else if (typeof config.params === "object" && !("user_id" in config.params)) {
  //     config.params = { ...(config.params as object), user_id: session.user_id };
  //   }
  // }
  if (session) {
    config.headers = {
      ...(config.headers || {}),
      "x-user-id": session.user_id || "",
      "x-open-game-id": session.open_game_id || "",
      "x-session-id": session.session_id || "",
    };
  }
  return config;
});

const dxNet = (method: string, extraHeaders?: Record<string, string>) => {
  return api.get('/api', {
    headers: {
      method,
      ...(extraHeaders || {}),
    },
  });
};
export { api, dxNet };
