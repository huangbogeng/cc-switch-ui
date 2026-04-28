import { type ProviderPreset, providerPresets } from '../../config/providerPresets';

interface Props {
  onSelect: (preset: ProviderPreset) => void;
  selectedId?: string;
}

export function PresetSelector({ onSelect, selectedId }: Props) {
  return (
    <div style={styles.grid}>
      {providerPresets.map((preset) => (
        <button
          key={preset.id}
          onClick={() => onSelect(preset)}
          style={{
            ...styles.card,
            borderColor: selectedId === preset.id ? preset.iconColor : '#0f3460',
            background: selectedId === preset.id ? `${preset.iconColor}20` : '#16213e',
          }}
        >
          <div style={styles.iconContainer}>
            <span style={{ ...styles.icon, background: preset.iconColor }}>
              {preset.name[0]}
            </span>
          </div>
          <div style={styles.info}>
            <span style={styles.name}>{preset.name}</span>
            {preset.description && (
              <span style={styles.description}>{preset.description}</span>
            )}
          </div>
          {preset.requiresOAuth && (
            <span style={styles.badge}>OAuth</span>
          )}
        </button>
      ))}
    </div>
  );
}

const styles: Record<string, React.CSSProperties> = {
  grid: {
    display: 'grid',
    gridTemplateColumns: 'repeat(auto-fill, minmax(200px, 1fr))',
    gap: '12px',
    marginBottom: '20px',
  },
  card: {
    display: 'flex',
    flexDirection: 'column',
    alignItems: 'center',
    padding: '20px 16px',
    borderRadius: '8px',
    border: '2px solid',
    cursor: 'pointer',
    transition: 'all 0.2s',
    background: '#16213e',
    gap: '10px',
  },
  iconContainer: {
    width: '48px',
    height: '48px',
    borderRadius: '12px',
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
  },
  icon: {
    width: '40px',
    height: '40px',
    borderRadius: '10px',
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
    color: '#fff',
    fontWeight: 'bold',
    fontSize: '18px',
  },
  info: {
    display: 'flex',
    flexDirection: 'column',
    alignItems: 'center',
    gap: '4px',
  },
  name: {
    fontWeight: 'bold',
    fontSize: '1em',
    color: '#eee',
  },
  description: {
    fontSize: '0.8em',
    color: '#888',
    textAlign: 'center',
  },
  badge: {
    fontSize: '0.7em',
    background: '#0f3460',
    color: '#00d4ff',
    padding: '2px 8px',
    borderRadius: '10px',
  },
};
