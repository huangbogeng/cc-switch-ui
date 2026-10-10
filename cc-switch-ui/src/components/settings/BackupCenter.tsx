import { useState } from 'react';
import { DatabaseBackupsPanel } from './DatabaseBackupsPanel';
import { FileBackupsPanel } from './FileBackupsPanel';
import { SkillBackupsPanel } from '@/components/skills/SkillBackupsPanel';
import { cacheDelete } from '@/lib/fetchCache';

export function BackupCenter() {
  const [recovering, setRecovering] = useState(false);
  return (
    <section id="backup-center" aria-labelledby="backup-center-title" className="space-y-4">
      <h2 id="backup-center-title" className="text-sm font-semibold">Backup Center</h2>
      <p className="text-xs leading-5 text-muted-foreground">Each backup has a different scope. Database restoration requires a restart; file snapshots retain original configuration; Skills recovery restores source files and syncs enabled skills.</p>
      <DatabaseBackupsPanel />
      <FileBackupsPanel />
      <SkillBackupsPanel disabled={false} onBusyChange={setRecovering} onRestored={() => { cacheDelete('skills'); }} />
      <p className="text-xs text-muted-foreground">{recovering ? 'Recovering skill files...' : 'Review restored skill records or retry a failed sync on the '} {!recovering && <a href="#skills" className="text-primary underline">Skills page</a>}.</p>
    </section>
  );
}
