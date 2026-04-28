/**
 * Provider Presets for Claude Code
 *
 * 迁移自 src.bak/config/claudeProviderPresets.ts
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
  requiresOAuth?: boolean;
  apiFormat?: 'anthropic' | 'openai_responses';
}

export const providerPresets: ProviderPreset[] = [
  {
    id: 'minimax',
    name: 'MiniMax',
    websiteUrl: 'https://platform.minimaxi.com',
    apiKeyUrl: 'https://platform.minimaxi.com/subscribe/coding-plan',
    settingsConfig: {
      env: {
        ANTHROPIC_BASE_URL: 'https://api.minimaxi.com/anthropic',
        ANTHROPIC_AUTH_TOKEN: '',
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
    description: 'MiniMax M2.7 模型',
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
        ANTHROPIC_MODEL: 'Pro/MiniMaxAI/MiniMax-M2.7',
        ANTHROPIC_DEFAULT_HAIKU_MODEL: 'Pro/MiniMaxAI/MiniMax-M2.7',
        ANTHROPIC_DEFAULT_SONNET_MODEL: 'Pro/MiniMaxAI/MiniMax-M2.7',
        ANTHROPIC_DEFAULT_OPUS_MODEL: 'Pro/MiniMaxAI/MiniMax-M2.7',
      },
    },
    icon: 'siliconflow',
    iconColor: '#6E29F6',
    description: 'SiliconFlow 聚合平台',
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
      },
    },
    modelsUrl: 'https://api.deepseek.com/models',
    icon: 'deepseek',
    iconColor: '#1E88E5',
    description: 'DeepSeek V4 模型',
  },
  {
    id: 'codex',
    name: 'Codex',
    websiteUrl: 'https://openai.com/chatgpt/pricing',
    settingsConfig: {
      env: {
        ANTHROPIC_BASE_URL: 'https://chatgpt.com/backend-api/codex',
        ANTHROPIC_MODEL: 'gpt-5.4',
        ANTHROPIC_DEFAULT_HAIKU_MODEL: 'gpt-5.4-mini',
        ANTHROPIC_DEFAULT_SONNET_MODEL: 'gpt-5.4',
        ANTHROPIC_DEFAULT_OPUS_MODEL: 'gpt-5.4',
      },
    },
    icon: 'openai',
    iconColor: '#000000',
    description: 'OpenAI Codex (需要 OAuth)',
    requiresOAuth: true,
    apiFormat: 'openai_responses',
  },
];
