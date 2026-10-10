import { useState } from 'react';
import { Server } from 'lucide-react';
import { providerLogos } from '@/lib/providerLogo';

export function ProviderLogo({ icon }: { icon?: string }) {
  const file = icon && Object.hasOwn(providerLogos, icon) ? providerLogos[icon] : undefined;
  const [failedFile, setFailedFile] = useState<string>();
  return <span aria-hidden="true" className="flex h-10 w-10 shrink-0 items-center justify-center rounded-xl border border-white/[0.07] bg-white/[0.025]">
    {file && failedFile !== file ? <img src={`${import.meta.env.BASE_URL}logos/${file}`} alt="" width={28} height={28}
      className="h-7 w-7 object-contain" decoding="async" onError={() => setFailedFile(file)} />
      : <Server className="h-5 w-5 text-muted-foreground" />}
  </span>;
}
