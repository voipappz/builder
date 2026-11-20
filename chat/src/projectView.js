// What the right pane and the empty chat show for the project Builder runs,
// by its builder.toml kind: a voice bot (talk to it), a product (its page),
// or a native app such as the phone (its page, e.g. the emulator's stream).

const APP_SUGGESTIONS = [
  { label: 'Explain this app', prompt: 'Explain how this app is organized: its modules, where the screens and the SIP code live, and how to run its tests.' },
  { label: 'Run the tests', prompt: 'Run make test and summarize the result.' },
  { label: 'Change the sign-in screen', prompt: 'Show me the sign-in screen code and suggest a clearer layout, then make the change and run make test.' }
];

const BOT_SUGGESTIONS = [
  { label: 'Explain this bot', prompt: 'Explain how this voice bot works: its pipeline, where the prompt and greeting live, and how to run its tests.' },
  { label: 'Change the greeting', prompt: 'Change the greeting to a short, friendly English greeting, run make test, and tell me when to restart the bot.' },
  { label: 'Run the tests', prompt: 'Run make test for this bot and summarize the result.' }
];

const PRODUCT_SUGGESTIONS = [
  { label: 'Explain this product', prompt: 'Explain how this product is organized: its main screens and services, and how to run its tests.' },
  { label: 'Run the tests', prompt: 'Run make test and summarize the result.' },
  { label: 'Find a screen', prompt: 'Find the component that renders the main screen and tell me its file and line.' }
];

export function projectView(bot, workspaceName) {
  if (!bot) return null;
  const kind = bot.kind === 'product' || bot.kind === 'app' ? bot.kind : 'bot';
  const canTalk = kind === 'bot';
  const pageUrl = canTalk ? null : bot.pageUrl || null;
  const noun = kind;
  const description = canTalk
    ? `The agent works on the ${bot.name} bot in ${workspaceName}. Talk to it in the Bot tab.`
    : pageUrl
      ? `The agent works on ${bot.name} in ${workspaceName}. See it in the ${bot.name} tab; tests and actions are in Status.`
      : `The agent works on ${bot.name} in ${workspaceName}. Tests and actions are in Status.`;
  return {
    kind,
    noun,
    canTalk,
    pageUrl,
    statusTab: canTalk && !pageUrl ? 'Bot' : 'Status',
    logLabel: `${noun[0].toUpperCase()}${noun.slice(1)} log`,
    description,
    suggestions: kind === 'app' ? APP_SUGGESTIONS : kind === 'product' ? PRODUCT_SUGGESTIONS : BOT_SUGGESTIONS
  };
}

/** Wait before reconnect attempt `attempt` (0-based): 1 s, 2 s, 4 s, then 5 s. */
export function reconnectDelay(attempt) {
  return Math.min(1000 * 2 ** attempt, 5000);
}
