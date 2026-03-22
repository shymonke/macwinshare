import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import './App.css';

interface Config {
  machine_id: string;
  machine_name: string;
  port: number;
  screen: {
    position: { x: number; y: number };
    width: number;
    height: number;
    edge_threshold: number;
    neighbors: Array<{ name: string; direction: string }>;
  };
  security: {
    tls_enabled: boolean;
    verify_clients: boolean;
    trusted_fingerprints: string[];
    password: string | null;
  };
  network: {
    mdns_enabled: boolean;
    udp_discovery_enabled: boolean;
    bind_interface: string;
    timeout_seconds: number;
  };
  clipboard: {
    enabled: boolean;
    share_text: boolean;
    share_images: boolean;
    share_files: boolean;
    max_size_bytes: number;
  };
  data_dir: string;
}

interface AppStatus {
  mode: string;
  connected: boolean;
  peer_name: string | null;
  fingerprint: string;
}

interface PeerInfo {
  name: string;
  address: string;
  fingerprint: string | null;
}

interface DisplayInfo {
  id: number;
  name: string;
  x: number;
  y: number;
  width: number;
  height: number;
  is_primary: boolean;
}

type Tab = 'status' | 'layout' | 'settings';

function App() {
  const [activeTab, setActiveTab] = useState<Tab>('status');
  const [config, setConfig] = useState<Config | null>(null);
  const [status, setStatus] = useState<AppStatus | null>(null);
  const [peers, setPeers] = useState<PeerInfo[]>([]);
  const [displays, setDisplays] = useState<DisplayInfo[]>([]);
  const [hasAccessibility, setHasAccessibility] = useState(true);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    loadData();
  }, []);

  async function loadData() {
    try {
      setLoading(true);
      const [configData, statusData, displayData, accessibilityData] = await Promise.all([
        invoke<Config>('get_config'),
        invoke<AppStatus>('get_status'),
        invoke<DisplayInfo[]>('get_displays'),
        invoke<boolean>('check_accessibility'),
      ]);
      setConfig(configData);
      setStatus(statusData);
      setDisplays(displayData);
      setHasAccessibility(accessibilityData);
      setError(null);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }

  async function discoverPeers() {
    try {
      const discovered = await invoke<PeerInfo[]>('discover_peers');
      setPeers(discovered);
    } catch (e) {
      setError(String(e));
    }
  }

  async function startServer() {
    try {
      await invoke('start_server');
      await loadData();
    } catch (e) {
      setError(String(e));
    }
  }

  async function startClient(serverAddr?: string) {
    try {
      await invoke('start_client', { serverAddr });
      await loadData();
    } catch (e) {
      setError(String(e));
    }
  }

  async function stop() {
    try {
      await invoke('stop');
      await loadData();
    } catch (e) {
      setError(String(e));
    }
  }

  if (loading) {
    return (
      <div className="app loading">
        <div className="spinner"></div>
        <p>Loading MacWinShare...</p>
      </div>
    );
  }

  return (
    <div className="app">
      <header className="header">
        <h1>MacWinShare</h1>
        <nav className="tabs">
          <button
            className={activeTab === 'status' ? 'active' : ''}
            onClick={() => setActiveTab('status')}
          >
            Status
          </button>
          <button
            className={activeTab === 'layout' ? 'active' : ''}
            onClick={() => setActiveTab('layout')}
          >
            Layout
          </button>
          <button
            className={activeTab === 'settings' ? 'active' : ''}
            onClick={() => setActiveTab('settings')}
          >
            Settings
          </button>
        </nav>
      </header>

      {error && (
        <div className="error-banner">
          <span>{error}</span>
          <button onClick={() => setError(null)}>Dismiss</button>
        </div>
      )}

      {!hasAccessibility && (
        <div className="warning-banner">
          <span>Accessibility permission required for input capture.</span>
          <button onClick={() => invoke('check_accessibility')}>
            Check Again
          </button>
        </div>
      )}

      <main className="content">
        {activeTab === 'status' && (
          <StatusPanel
            status={status}
            config={config}
            peers={peers}
            onStartServer={startServer}
            onStartClient={startClient}
            onStop={stop}
            onDiscoverPeers={discoverPeers}
          />
        )}

        {activeTab === 'layout' && (
          <LayoutPanel displays={displays} config={config} />
        )}

        {activeTab === 'settings' && (
          <SettingsPanel config={config} onSave={loadData} />
        )}
      </main>
    </div>
  );
}

interface StatusPanelProps {
  status: AppStatus | null;
  config: Config | null;
  peers: PeerInfo[];
  onStartServer: () => void;
  onStartClient: (addr?: string) => void;
  onStop: () => void;
  onDiscoverPeers: () => void;
}

function StatusPanel({
  status,
  config,
  peers,
  onStartServer,
  onStartClient,
  onStop,
  onDiscoverPeers,
}: StatusPanelProps) {
  return (
    <div className="status-panel">
      <section className="status-info">
        <h2>Connection Status</h2>
        <div className="status-grid">
          <div className="status-item">
            <span className="label">Machine Name</span>
            <span className="value">{config?.machine_name || 'Unknown'}</span>
          </div>
          <div className="status-item">
            <span className="label">Mode</span>
            <span className={`value mode-${status?.mode.toLowerCase()}`}>
              {status?.mode || 'Idle'}
            </span>
          </div>
          <div className="status-item">
            <span className="label">Connected</span>
            <span className={`value ${status?.connected ? 'connected' : 'disconnected'}`}>
              {status?.connected ? 'Yes' : 'No'}
            </span>
          </div>
          <div className="status-item">
            <span className="label">Port</span>
            <span className="value">{config?.port || 24800}</span>
          </div>
        </div>

        <div className="fingerprint-section">
          <h3>Certificate Fingerprint</h3>
          <code className="fingerprint">{status?.fingerprint || 'Not available'}</code>
          <p className="fingerprint-hint">
            Share this fingerprint with peers to verify secure connections.
          </p>
        </div>
      </section>

      <section className="actions">
        <h2>Quick Actions</h2>
        <div className="action-buttons">
          <button className="btn primary" onClick={onStartServer}>
            Start as Server
          </button>
          <button className="btn secondary" onClick={() => onStartClient()}>
            Connect as Client
          </button>
          <button className="btn danger" onClick={onStop}>
            Stop
          </button>
        </div>
      </section>

      <section className="peers">
        <h2>
          Discovered Peers
          <button className="btn small" onClick={onDiscoverPeers}>
            Refresh
          </button>
        </h2>
        {peers.length === 0 ? (
          <p className="no-peers">No peers discovered yet. Click Refresh to scan.</p>
        ) : (
          <ul className="peer-list">
            {peers.map((peer, i) => (
              <li key={i} className="peer-item">
                <div className="peer-info">
                  <span className="peer-name">{peer.name}</span>
                  <span className="peer-address">{peer.address}</span>
                </div>
                <button
                  className="btn small"
                  onClick={() => onStartClient(peer.address)}
                >
                  Connect
                </button>
              </li>
            ))}
          </ul>
        )}
      </section>
    </div>
  );
}

interface LayoutPanelProps {
  displays: DisplayInfo[];
  config: Config | null;
}

function LayoutPanel({ displays, config }: LayoutPanelProps) {
  // Calculate bounds for scaling
  const allDisplays = displays.length > 0 ? displays : [
    { id: 0, name: 'Display 1', x: 0, y: 0, width: 1920, height: 1080, is_primary: true },
  ];

  const minX = Math.min(...allDisplays.map(d => d.x));
  const minY = Math.min(...allDisplays.map(d => d.y));
  const maxX = Math.max(...allDisplays.map(d => d.x + d.width));
  const maxY = Math.max(...allDisplays.map(d => d.y + d.height));
  const totalWidth = maxX - minX;
  const totalHeight = maxY - minY;

  const scale = Math.min(600 / totalWidth, 400 / totalHeight, 0.15);

  return (
    <div className="layout-panel">
      <h2>Monitor Layout</h2>
      <p className="layout-hint">
        Drag monitors to arrange them. Connected peers will appear as additional screens.
      </p>

      <div className="layout-canvas" style={{ width: totalWidth * scale + 40, height: totalHeight * scale + 40 }}>
        {allDisplays.map((display) => (
          <div
            key={display.id}
            className={`display-box ${display.is_primary ? 'primary' : ''}`}
            style={{
              left: (display.x - minX) * scale + 20,
              top: (display.y - minY) * scale + 20,
              width: display.width * scale,
              height: display.height * scale,
            }}
          >
            <span className="display-name">{display.name}</span>
            <span className="display-resolution">
              {display.width}x{display.height}
            </span>
            {display.is_primary && <span className="primary-badge">Primary</span>}
          </div>
        ))}

        {config?.screen.neighbors.map((neighbor, i) => (
          <div
            key={`neighbor-${i}`}
            className="display-box neighbor"
            style={{
              left: neighbor.direction === 'Right' ? totalWidth * scale + 30 : -150,
              top: totalHeight * scale / 2 - 50,
              width: 140,
              height: 100,
            }}
          >
            <span className="display-name">{neighbor.name}</span>
            <span className="neighbor-badge">{neighbor.direction}</span>
          </div>
        ))}
      </div>

      <section className="layout-info">
        <h3>Current Screen</h3>
        <div className="info-grid">
          <div className="info-item">
            <span className="label">Position</span>
            <span className="value">
              ({config?.screen.position.x || 0}, {config?.screen.position.y || 0})
            </span>
          </div>
          <div className="info-item">
            <span className="label">Size</span>
            <span className="value">
              {config?.screen.width || 1920} x {config?.screen.height || 1080}
            </span>
          </div>
          <div className="info-item">
            <span className="label">Edge Threshold</span>
            <span className="value">{config?.screen.edge_threshold || 1}px</span>
          </div>
        </div>
      </section>
    </div>
  );
}

interface SettingsPanelProps {
  config: Config | null;
  onSave: () => void;
}

function SettingsPanel({ config, onSave }: SettingsPanelProps) {
  const [localConfig, setLocalConfig] = useState<Config | null>(config);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    setLocalConfig(config);
  }, [config]);

  async function handleSave() {
    if (!localConfig) return;
    setSaving(true);
    try {
      await invoke('save_config', { newConfig: localConfig });
      onSave();
    } catch (e) {
      console.error('Failed to save config:', e);
    } finally {
      setSaving(false);
    }
  }

  if (!localConfig) {
    return <div>Loading settings...</div>;
  }

  return (
    <div className="settings-panel">
      <h2>Settings</h2>

      <section className="settings-section">
        <h3>General</h3>
        <div className="setting-item">
          <label htmlFor="machine-name">Machine Name</label>
          <input
            id="machine-name"
            type="text"
            value={localConfig.machine_name}
            onChange={(e) =>
              setLocalConfig({ ...localConfig, machine_name: e.target.value })
            }
          />
        </div>
        <div className="setting-item">
          <label htmlFor="port">Port</label>
          <input
            id="port"
            type="number"
            value={localConfig.port}
            onChange={(e) =>
              setLocalConfig({ ...localConfig, port: parseInt(e.target.value) || 24800 })
            }
          />
        </div>
      </section>

      <section className="settings-section">
        <h3>Security</h3>
        <div className="setting-item checkbox">
          <input
            id="tls-enabled"
            type="checkbox"
            checked={localConfig.security.tls_enabled}
            onChange={(e) =>
              setLocalConfig({
                ...localConfig,
                security: { ...localConfig.security, tls_enabled: e.target.checked },
              })
            }
          />
          <label htmlFor="tls-enabled">Enable TLS Encryption</label>
        </div>
        <div className="setting-item checkbox">
          <input
            id="verify-clients"
            type="checkbox"
            checked={localConfig.security.verify_clients}
            onChange={(e) =>
              setLocalConfig({
                ...localConfig,
                security: { ...localConfig.security, verify_clients: e.target.checked },
              })
            }
          />
          <label htmlFor="verify-clients">Verify Client Certificates</label>
        </div>
      </section>

      <section className="settings-section">
        <h3>Network Discovery</h3>
        <div className="setting-item checkbox">
          <input
            id="mdns-enabled"
            type="checkbox"
            checked={localConfig.network.mdns_enabled}
            onChange={(e) =>
              setLocalConfig({
                ...localConfig,
                network: { ...localConfig.network, mdns_enabled: e.target.checked },
              })
            }
          />
          <label htmlFor="mdns-enabled">Enable mDNS Discovery</label>
        </div>
        <div className="setting-item checkbox">
          <input
            id="udp-enabled"
            type="checkbox"
            checked={localConfig.network.udp_discovery_enabled}
            onChange={(e) =>
              setLocalConfig({
                ...localConfig,
                network: { ...localConfig.network, udp_discovery_enabled: e.target.checked },
              })
            }
          />
          <label htmlFor="udp-enabled">Enable UDP Broadcast Discovery</label>
        </div>
      </section>

      <section className="settings-section">
        <h3>Clipboard</h3>
        <div className="setting-item checkbox">
          <input
            id="clipboard-enabled"
            type="checkbox"
            checked={localConfig.clipboard.enabled}
            onChange={(e) =>
              setLocalConfig({
                ...localConfig,
                clipboard: { ...localConfig.clipboard, enabled: e.target.checked },
              })
            }
          />
          <label htmlFor="clipboard-enabled">Enable Clipboard Sharing</label>
        </div>
        <div className="setting-item checkbox">
          <input
            id="share-text"
            type="checkbox"
            checked={localConfig.clipboard.share_text}
            disabled={!localConfig.clipboard.enabled}
            onChange={(e) =>
              setLocalConfig({
                ...localConfig,
                clipboard: { ...localConfig.clipboard, share_text: e.target.checked },
              })
            }
          />
          <label htmlFor="share-text">Share Text</label>
        </div>
        <div className="setting-item checkbox">
          <input
            id="share-images"
            type="checkbox"
            checked={localConfig.clipboard.share_images}
            disabled={!localConfig.clipboard.enabled}
            onChange={(e) =>
              setLocalConfig({
                ...localConfig,
                clipboard: { ...localConfig.clipboard, share_images: e.target.checked },
              })
            }
          />
          <label htmlFor="share-images">Share Images</label>
        </div>
        <div className="setting-item checkbox">
          <input
            id="share-files"
            type="checkbox"
            checked={localConfig.clipboard.share_files}
            disabled={!localConfig.clipboard.enabled}
            onChange={(e) =>
              setLocalConfig({
                ...localConfig,
                clipboard: { ...localConfig.clipboard, share_files: e.target.checked },
              })
            }
          />
          <label htmlFor="share-files">Share Files</label>
        </div>
      </section>

      <div className="settings-actions">
        <button className="btn primary" onClick={handleSave} disabled={saving}>
          {saving ? 'Saving...' : 'Save Settings'}
        </button>
        <button className="btn secondary" onClick={() => setLocalConfig(config)}>
          Reset
        </button>
      </div>
    </div>
  );
}

export default App;
