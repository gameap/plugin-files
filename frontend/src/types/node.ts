export type NodeStatus = 'not_installed' | 'installing' | 'installed' | 'failed';

export interface NodeSetupStatus {
  status: NodeStatus;
  version?: string;
  task_id?: number;
  error_message?: string;
  last_check: number;
}

export interface FTPConfig {
  address?: string;
  port?: number;
  passive_port_min?: number;
  passive_port_max?: number;
  public_host?: string;
  tls_enabled?: boolean;
  tls_implicit_port?: number;
}

export interface SFTPConfig {
  port?: number;
}

export interface NodeSetupConfig {
  ftp?: FTPConfig;
  sftp?: SFTPConfig;
}

// Response from GET /nodes/{nodeId}/config - all fields present
export interface FTPConfigResponse {
  address: string;
  port: number;
  passive_port_min: number;
  passive_port_max: number;
  public_host: string;
  tls_enabled: boolean;
  tls_implicit_port: number;
}

export interface SFTPConfigResponse {
  port: number;
}

export interface NodeConfigResponse {
  ftp: FTPConfigResponse;
  sftp: SFTPConfigResponse;
}
