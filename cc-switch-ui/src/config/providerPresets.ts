/**
 * Provider Presets for Claude Code
 *
 * Migrated from src.bak/config/claudeProviderPresets.ts
 */

export interface ProviderPreset {
  id: string;
  name: string;
  websiteUrl: string;
  apiKeyUrl?: string;
  settingsConfig: {
    env: Record<string, string>;
    apiTimeoutMs?: string;
    claudeCodeDisableNonessentialTraffic?: string;
  };
  modelsUrl?: string;
  icon: string;
  iconColor: string;
  description?: string;
  authMode?: 'api_key' | 'oauth_proxy';
  /**
   * Provider needs no credentials of its own (e.g. the official Claude
   * subscription login). The form hides the API-key/base-url fields, skips the
   * missing-key confirmation, and saves an empty env so switching to it clears
   * any third-party takeover and falls back to the logged-in subscription.
   */
  noAuth?: boolean;
  providerType?: 'codex_oauth';
  requiresOAuth?: boolean;
  codexFastMode?: boolean;
  apiKeyField?: 'ANTHROPIC_AUTH_TOKEN' | 'ANTHROPIC_API_KEY';
  apiFormat?: 'anthropic' | 'openai_chat' | 'openai_responses' | 'gemini_native';
  isFullUrl?: boolean;
}

export const providerPresets: ProviderPreset[] = [
  {
    id: 'claude-official',
    name: 'Claude Official',
    websiteUrl: 'https://www.anthropic.com/claude-code',
    settingsConfig: {
      env: {},
    },
    icon: 'anthropic',
    iconColor: '#D4915D',
    description: 'Official Anthropic subscription (clears third-party provider settings)',
    noAuth: true,
  },
  {
    id: 'minimax',
    name: 'MiniMax',
    websiteUrl: 'https://platform.minimaxi.com',
    apiKeyUrl: 'https://platform.minimaxi.com/subscribe/coding-plan',
    settingsConfig: {
      env: {
        ANTHROPIC_BASE_URL: 'https://api.minimaxi.com/v1',
        ANTHROPIC_AUTH_TOKEN: '',
        CLAUDE_CODE_AUTO_MODE_SERVER: '0',
        API_TIMEOUT_MS: '3000000',
        CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC: '1',
        ANTHROPIC_MODEL: 'MiniMax-M2.7',
        ANTHROPIC_DEFAULT_SONNET_MODEL: 'MiniMax-M2.7',
        ANTHROPIC_DEFAULT_OPUS_MODEL: 'MiniMax-M2.7',
        ANTHROPIC_DEFAULT_HAIKU_MODEL: 'MiniMax-M2.7',
      },
    },
    icon: 'minimax',
    iconColor: '#FF6B6B',
    description: 'MiniMax M2.7 model',
    apiFormat: 'openai_chat',
  },
  {
    id: 'siliconflow',
    name: 'SiliconFlow',
    websiteUrl: 'https://siliconflow.cn',
    apiKeyUrl: 'https://cloud.siliconflow.cn/i/drGuwc9k',
    settingsConfig: {
      env: {
        ANTHROPIC_BASE_URL: 'https://api.siliconflow.cn',
        ANTHROPIC_AUTH_TOKEN: '',
        CLAUDE_CODE_AUTO_MODE_SERVER: '0',
        ANTHROPIC_MODEL: 'Pro/MiniMaxAI/MiniMax-M2.7',
        ANTHROPIC_DEFAULT_HAIKU_MODEL: 'Pro/MiniMaxAI/MiniMax-M2.7',
        ANTHROPIC_DEFAULT_SONNET_MODEL: 'Pro/MiniMaxAI/MiniMax-M2.7',
        ANTHROPIC_DEFAULT_OPUS_MODEL: 'Pro/MiniMaxAI/MiniMax-M2.7',
      },
    },
    icon: 'siliconflow',
    iconColor: '#6E29F6',
    description: 'SiliconFlow multi-model API platform',
  },
  {
    id: 'deepseek',
    name: 'DeepSeek',
    websiteUrl: 'https://platform.deepseek.com',
    settingsConfig: {
      env: {
        ANTHROPIC_BASE_URL: 'https://api.deepseek.com/anthropic',
        ANTHROPIC_AUTH_TOKEN: '',
        ANTHROPIC_MODEL: 'deepseek-v4-pro',
        ANTHROPIC_DEFAULT_HAIKU_MODEL: 'deepseek-v4-flash',
        ANTHROPIC_DEFAULT_SONNET_MODEL: 'deepseek-v4-pro',
        ANTHROPIC_DEFAULT_OPUS_MODEL: 'deepseek-v4-pro',
        CLAUDE_CODE_SUBAGENT_MODEL: 'deepseek-v4-flash',
        CLAUDE_CODE_EFFORT_LEVEL: 'max',
        CLAUDE_CODE_AUTO_MODE_SERVER: '0',
      },
    },
    modelsUrl: 'https://api.deepseek.com/models',
    icon: 'deepseek',
    iconColor: '#1E88E5',
    description: 'DeepSeek V4 models',
  },
  {
    id: 'codex',
    name: 'Codex',
    websiteUrl: 'https://openai.com/chatgpt/pricing',
    settingsConfig: {
      env: {
        ANTHROPIC_BASE_URL: 'https://chatgpt.com/backend-api/codex',
        ANTHROPIC_MODEL: 'gpt-6.1-sol',
        ANTHROPIC_DEFAULT_HAIKU_MODEL: 'gpt-6.1-sol',
        ANTHROPIC_DEFAULT_SONNET_MODEL: 'gpt-6.1-sol',
        ANTHROPIC_DEFAULT_OPUS_MODEL: 'gpt-6.1-sol',
      },
    },
    icon: 'openai',
    iconColor: '#000000',
    description: 'OpenAI Codex account through OAuth + local route',
    authMode: 'oauth_proxy',
    providerType: 'codex_oauth',
    requiresOAuth: true,
    apiFormat: 'openai_responses',
  },
  {
    id: 'openrouter',
    name: 'OpenRouter',
    websiteUrl: 'https://openrouter.ai',
    apiKeyUrl: 'https://openrouter.ai/keys',
    settingsConfig: {
      env: {
        ANTHROPIC_BASE_URL: 'https://openrouter.ai/api',
        ANTHROPIC_AUTH_TOKEN: '',
        CLAUDE_CODE_AUTO_MODE_SERVER: '0',
        ANTHROPIC_MODEL: 'anthropic/claude-sonnet-4.6',
        ANTHROPIC_DEFAULT_HAIKU_MODEL: 'anthropic/claude-haiku-4.5',
        ANTHROPIC_DEFAULT_SONNET_MODEL: 'anthropic/claude-sonnet-4.6',
        ANTHROPIC_DEFAULT_OPUS_MODEL: 'anthropic/claude-opus-4.7',
      },
    },
    icon: 'openrouter',
    iconColor: '#7C3AED',
    description: 'Access 100+ AI models via unified API',
  },
  {
    id: 'gemini-native',
    name: 'Gemini Native',
    websiteUrl: 'https://ai.google.dev',
    apiKeyUrl: 'https://aistudio.google.com/app/apikey',
    settingsConfig: {
      env: {
        ANTHROPIC_BASE_URL: 'https://generativelanguage.googleapis.com',
        ANTHROPIC_API_KEY: '',
        ANTHROPIC_MODEL: 'gemini-3.1-pro',
        ANTHROPIC_DEFAULT_HAIKU_MODEL: 'gemini-3-flash',
        ANTHROPIC_DEFAULT_SONNET_MODEL: 'gemini-3.1-pro',
        ANTHROPIC_DEFAULT_OPUS_MODEL: 'gemini-3.1-pro',
      },
    },
    icon: 'google',
    iconColor: '#4285F4',
    description: 'Google Gemini Native API (gemini_native format)',
    apiKeyField: 'ANTHROPIC_API_KEY',
    apiFormat: 'gemini_native',
  },
  {
    id: 'kimi',
    name: 'Kimi',
    websiteUrl: 'https://platform.kimi.com',
    apiKeyUrl: 'https://platform.kimi.com',
    settingsConfig: {
      env: {
        ANTHROPIC_BASE_URL: 'https://api.moonshot.cn/anthropic',
        ANTHROPIC_AUTH_TOKEN: '',
        CLAUDE_CODE_AUTO_MODE_SERVER: '0',
        ANTHROPIC_MODEL: 'kimi-k2.7-code',
        ANTHROPIC_DEFAULT_HAIKU_MODEL: 'kimi-k2.7-code',
        ANTHROPIC_DEFAULT_SONNET_MODEL: 'kimi-k2.7-code',
        ANTHROPIC_DEFAULT_OPUS_MODEL: 'kimi-k2.7-code',
      },
    },
    icon: 'kimi',
    iconColor: '#6366F1',
    description: 'Moonshot Kimi K2.7 (pay as you go)',
  },
  {
    id: 'kimi-for-coding',
    name: 'Kimi For Coding',
    websiteUrl: 'https://www.kimi.com/code/',
    apiKeyUrl: 'https://www.kimi.com/code/',
    settingsConfig: {
      env: {
        ANTHROPIC_BASE_URL: 'https://api.kimi.com/coding/',
        ANTHROPIC_AUTH_TOKEN: '',
        CLAUDE_CODE_AUTO_MODE_SERVER: '0',
        ANTHROPIC_MODEL: 'kimi-for-coding',
        ANTHROPIC_DEFAULT_HAIKU_MODEL: 'kimi-for-coding',
        ANTHROPIC_DEFAULT_SONNET_MODEL: 'kimi-for-coding',
        ANTHROPIC_DEFAULT_OPUS_MODEL: 'kimi-for-coding',
        CLAUDE_CODE_MAX_CONTEXT_TOKENS: '262144',
        CLAUDE_CODE_AUTO_COMPACT_WINDOW: '262144',
      },
    },
    icon: 'kimi',
    iconColor: '#6366F1',
    description: 'Kimi Coding Plan (256K context)',
  },
  {
    id: 'kimi-for-coding-global',
    name: 'Kimi For Coding Global',
    websiteUrl: 'https://www.kimi.ai/code',
    apiKeyUrl: 'https://www.kimi.ai/code',
    settingsConfig: {
      env: {
        ANTHROPIC_BASE_URL: 'https://api.kimi.ai/coding/',
        ANTHROPIC_AUTH_TOKEN: '',
        CLAUDE_CODE_AUTO_MODE_SERVER: '0',
        ANTHROPIC_MODEL: 'kimi-for-coding',
        ANTHROPIC_DEFAULT_HAIKU_MODEL: 'kimi-for-coding',
        ANTHROPIC_DEFAULT_SONNET_MODEL: 'kimi-for-coding',
        ANTHROPIC_DEFAULT_OPUS_MODEL: 'kimi-for-coding',
        CLAUDE_CODE_MAX_CONTEXT_TOKENS: '262144',
        CLAUDE_CODE_AUTO_COMPACT_WINDOW: '262144',
      },
    },
    icon: 'kimi',
    iconColor: '#6366F1',
    description: 'Kimi Coding Plan Global (api.kimi.ai)',
  },
  {
    id: 'zhipu-glm',
    name: 'Zhipu GLM',
    websiteUrl: 'https://open.bigmodel.cn',
    apiKeyUrl: 'https://www.bigmodel.cn/claude-code',
    settingsConfig: {
      env: {
        ANTHROPIC_BASE_URL: 'https://open.bigmodel.cn/api/anthropic',
        ANTHROPIC_AUTH_TOKEN: '',
        CLAUDE_CODE_AUTO_MODE_SERVER: '0',
        ANTHROPIC_MODEL: 'glm-5.3',
        ANTHROPIC_DEFAULT_HAIKU_MODEL: 'glm-5.3',
        ANTHROPIC_DEFAULT_SONNET_MODEL: 'glm-5.3',
        ANTHROPIC_DEFAULT_OPUS_MODEL: 'glm-5.3',
      },
    },
    icon: 'zhipu',
    iconColor: '#0F62FE',
    description: 'Zhipu GLM-5.3 (China)',
  },
  {
    id: 'zhipu-glm-global',
    name: 'Zhipu GLM en',
    websiteUrl: 'https://z.ai',
    apiKeyUrl: 'https://z.ai/subscribe',
    settingsConfig: {
      env: {
        ANTHROPIC_BASE_URL: 'https://api.z.ai/api/anthropic',
        ANTHROPIC_AUTH_TOKEN: '',
        CLAUDE_CODE_AUTO_MODE_SERVER: '0',
        ANTHROPIC_MODEL: 'glm-5.3',
        ANTHROPIC_DEFAULT_HAIKU_MODEL: 'glm-5.3',
        ANTHROPIC_DEFAULT_SONNET_MODEL: 'glm-5.3',
        ANTHROPIC_DEFAULT_OPUS_MODEL: 'glm-5.3',
      },
    },
    icon: 'zhipu',
    iconColor: '#0F62FE',
    description: 'Zhipu GLM-5.3 (international, z.ai)',
  },
  {
    id: 'volcengine-coding-plan',
    name: 'Volcengine Coding Plan',
    websiteUrl: 'https://www.volcengine.com/activity/codingplan',
    apiKeyUrl: 'https://www.volcengine.com/activity/codingplan',
    settingsConfig: {
      env: {
        ANTHROPIC_BASE_URL: 'https://ark.cn-beijing.volces.com/api/coding',
        ANTHROPIC_AUTH_TOKEN: '',
        CLAUDE_CODE_AUTO_MODE_SERVER: '0',
        ANTHROPIC_MODEL: 'ark-code-latest',
        ANTHROPIC_DEFAULT_HAIKU_MODEL: 'ark-code-latest',
        ANTHROPIC_DEFAULT_SONNET_MODEL: 'ark-code-latest',
        ANTHROPIC_DEFAULT_OPUS_MODEL: 'ark-code-latest',
      },
    },
    icon: 'huoshan',
    iconColor: '#3370FF',
    description: 'Volcengine Ark Coding Plan',
  },
  {
    id: 'qianwen-coding-plan',
    name: 'Qwen Coding Plan',
    websiteUrl: 'https://bailian.console.aliyun.com',
    settingsConfig: {
      env: {
        ANTHROPIC_BASE_URL: 'https://coding.dashscope.aliyuncs.com/apps/anthropic',
        ANTHROPIC_AUTH_TOKEN: '',
        CLAUDE_CODE_AUTO_MODE_SERVER: '0',
      },
    },
    icon: 'qianwenai',
    iconColor: '#624AFF',
    description: 'Alibaba Cloud Model Studio Qwen Coding Plan',
  },
  {
    id: 'aihubmix',
    name: 'AiHubMix',
    websiteUrl: 'https://aihubmix.com',
    apiKeyUrl: 'https://aihubmix.com',
    settingsConfig: {
      env: {
        ANTHROPIC_BASE_URL: 'https://aihubmix.com',
        ANTHROPIC_API_KEY: '',
        CLAUDE_CODE_AUTO_MODE_SERVER: '0',
      },
    },
    icon: 'aihubmix',
    iconColor: '#006FFB',
    description: 'AiHubMix multi-model API platform (uses ANTHROPIC_API_KEY)',
    apiKeyField: 'ANTHROPIC_API_KEY',
  },
  {
    id: 'ppio',
    name: 'PPIO',
    websiteUrl: 'https://ppio.com',
    apiKeyUrl: 'https://ppio.com/activity/ccswitch',
    modelsUrl: 'https://api.ppio.com/openai/v1/models',
    settingsConfig: {
      env: {
        ANTHROPIC_BASE_URL: 'https://api.ppio.com/anthropic',
        ANTHROPIC_AUTH_TOKEN: '',
        CLAUDE_CODE_AUTO_MODE_SERVER: '0',
        ANTHROPIC_MODEL: 'deepseek/deepseek-v4-flash-0731',
        ANTHROPIC_DEFAULT_HAIKU_MODEL: 'deepseek/deepseek-v4-flash-0731',
        ANTHROPIC_DEFAULT_SONNET_MODEL: 'deepseek/deepseek-v4-flash-0731',
        ANTHROPIC_DEFAULT_OPUS_MODEL: 'deepseek/deepseek-v4-flash-0731',
      },
    },
    icon: 'ppio',
    iconColor: '#2874FF',
    description: 'PPIO multi-model API platform',
  },
];
