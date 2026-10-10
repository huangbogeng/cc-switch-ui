import type { Provider } from '@/api';
import { providerPresets } from '@/config/providerPresets';
import { providerBaseUrl } from './provider';

export const providerLogos: Record<string, string> = {
  anthropic: 'anthropic.svg', minimax: 'minimax.svg', siliconflow: 'siliconflow.svg',
  deepseek: 'deepseek.svg', openai: 'openai.svg', openrouter: 'openrouter.svg',
  orcarouter: 'orcarouter.png', google: 'google.svg', kimi: 'kimi.svg',
  zhipu: 'zhipu.svg', huoshan: 'huoshan.png', qianwenai: 'qianwenai.svg',
  aihubmix: 'aihubmix-color.svg', ppio: 'ppio.svg',
};

const domains: Record<string, string> = {
  'anthropic.com': 'anthropic', 'minimaxi.com': 'minimax', 'minimax.io': 'minimax',
  'siliconflow.cn': 'siliconflow', 'siliconflow.com': 'siliconflow',
  'deepseek.com': 'deepseek', 'openai.com': 'openai', 'chatgpt.com': 'openai',
  'openrouter.ai': 'openrouter', 'orcarouter.ai': 'orcarouter',
  'googleapis.com': 'google', 'moonshot.cn': 'kimi', 'moonshot.ai': 'kimi',
  'kimi.com': 'kimi', 'kimi.ai': 'kimi', 'bigmodel.cn': 'zhipu', 'z.ai': 'zhipu',
  'volces.com': 'huoshan', 'volcengine.com': 'huoshan',
  'dashscope.aliyuncs.com': 'qianwenai', 'bailian.console.aliyun.com': 'qianwenai',
  'aihubmix.com': 'aihubmix', 'ppio.com': 'ppio',
};

export function resolveProviderLogo(provider: Provider): string | undefined {
  if (provider.icon && Object.hasOwn(providerLogos, provider.icon)) return provider.icon;
  const preset = providerPresets.find((item) => item.id === provider.id);
  if (preset) return preset.icon;
  const meta = provider.meta as { providerType?: string; noAuth?: boolean } | null;
  if (meta?.providerType === 'codex_oauth') return 'openai';
  for (const url of [providerBaseUrl(provider), provider.websiteUrl]) {
    if (!url) continue;
    try {
      const parsed = new URL(url);
      if (!['http:', 'https:'].includes(parsed.protocol)) continue;
      const match = Object.entries(domains).find(([domain]) => parsed.hostname === domain || parsed.hostname.endsWith(`.${domain}`));
      if (match) return match[1];
    } catch { /* Custom endpoints use the neutral fallback. */ }
  }
  return undefined;
}
