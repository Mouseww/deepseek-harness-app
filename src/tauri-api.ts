/**
 * DSH Desktop - Tauri IPC 接口
 * 提供前端与 Rust 后端通信的 TypeScript 类型定义
 */

import { invoke } from '@tauri-apps/api/core'

export interface DshConfig {
  host: string
  port: number
  auto_start: boolean
}

export class DshBackendClient {
  /**
   * 启动 DSH 后端进程
   * @param config DSH 配置
   * @returns 实际监听的端口
   */
  static async start(config: DshConfig): Promise<number> {
    return await invoke<number>('start_dsh_backend', { config })
  }

  /**
   * 停止 DSH 后端进程
   */
  static async stop(): Promise<void> {
    await invoke('stop_dsh_backend')
  }

  /**
   * 获取 DSH 后端运行状态
   * @returns true 表示正在运行
   */
  static async getStatus(): Promise<boolean> {
    return await invoke<boolean>('get_dsh_status')
  }

  /**
   * 获取 DSH 当前监听端口
   * @returns 端口号，如果未运行则返回 null
   */
  static async getPort(): Promise<number | null> {
    return await invoke<number | null>('get_dsh_port')
  }

  /**
   * 获取配置
   */
  static async getConfig(): Promise<DshConfig> {
    return await invoke<DshConfig>('get_config')
  }

  /**
   * 保存配置
   */
  static async setConfig(config: DshConfig): Promise<void> {
    await invoke('set_config', { config })
  }

  /**
   * 检查更新
   * @returns true 表示有新版本可用
   */
  static async checkUpdates(): Promise<boolean> {
    return await invoke<boolean>('check_app_updates')
  }
}

/**
 * 检测是否在 Tauri 环境中运行
 */
export function isTauriEnvironment(): boolean {
  return '__TAURI_INTERNALS__' in window
}
