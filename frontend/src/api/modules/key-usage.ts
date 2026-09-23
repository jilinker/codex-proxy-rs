import type { RequestOptions } from '../request'
import type { AccountPersonalInfoResponse, AccountQuotaForecastResponse, AccountQuotaWindow, AccountResetCreditResultResponse, AccountResetCreditsResponse, AccountStatus } from './accounts'
import type { DashboardHealthTimeline } from './dashboard'
import type { UsageBilling, UsageLatencyDetails, UsageTokenDetails } from './usage'
import { API_BASE_URL } from '../constants'
import request from '../request'

export interface KeyUsageConfig {
  name: string
  plaintextKey: string
}

export interface KeyUsageVersion {
  version: string
  gitSha: string
}

export interface KeyUsageMetrics {
  requests: number
  inputTokens: number
  outputTokens: number
  cachedTokens: number
  cacheWriteTokens: number
  reasoningTokens: number
  totalTokens: number
  costUsd: string | null
  costIncomplete: boolean
}

export interface KeyUsageBudget {
  name: string
  prefix: string
  maxConcurrency: number
  requestsPerMinute: number
  dailyLimitUsd: string
  dailyUsedUsd: string
  dailyResetsAt: string | null
  weeklyLimitUsd: string
  weeklyUsedUsd: string
  weeklyResetsAt: string | null
}

export interface KeyUsageTrendPoint extends KeyUsageMetrics {
  time: string
  bucketSeconds: number
}

export interface KeyUsageOverview {
  asOf: string
  startTime: string
  endTime: string
  key: KeyUsageBudget
  summary: KeyUsageMetrics
  trend: KeyUsageTrendPoint[]
  healthTimeline: DashboardHealthTimeline
}

export type KeyUsageRecordKind = 'success' | 'error'

export interface KeyUsageRecord {
  id: string
  createdAt: string
  model: string | null
  route: string | null
  reasoningEffort: string | null
  clientTransport: string | null
  upstreamTransport: string | null
  tokenDetails: UsageTokenDetails | null
  billing: UsageBilling | null
  latencyMs: number | null
  firstTokenLatencyMs: number | null
  latencyDetails: Pick<UsageLatencyDetails, 'firstEventMs' | 'firstReasoningMs' | 'firstTextMs'>
  clientIp: string | null
  userAgent: string | null
  status: KeyUsageRecordKind
  statusCode: number | null
}

export interface KeyUsagePage {
  items: KeyUsageRecord[]
  currentPage: number
  pageSize: number
  total: number
}

export type KeyUsageAccountScopeState = 'unbound' | 'no_enabled_groups' | 'available'
export type KeyUsageQuotaAvailability = 'available' | 'unsupported' | 'unobserved' | 'unavailable'

export interface KeyUsageAccountQuotaWindow {
  key: string
  group: string
  limitId: string | null
  limitName: string | null
  role: AccountQuotaWindow['role']
  windowSeconds: number | null
  labelDisplay: string
  windowLabelDisplay: string
  usedPercent: number | null
  usedPercentDisplay: string
  limitReached: boolean
  localUsage: {
    requestCount: number
    requestCountDisplay: string
    inputTokens: number | null
    inputTokensDisplay: string
    outputTokens: number | null
    outputTokensDisplay: string
    cachedTokens: number | null
    cachedTokensDisplay: string
    totalTokens: number | null
    totalTokensDisplay: string
    requestBuckets: { bucketStart: string, requestCount: number }[]
  } | null
  resetAtDisplay: string
}

export interface KeyUsageAccountQuota {
  availability: KeyUsageQuotaAvailability
  refreshedAtDisplay: string
  limitReached: boolean
  windows: KeyUsageAccountQuotaWindow[]
}

export interface KeyUsageAccountUsageSummary {
  recentModel: { model: string, lastUsedAt: string } | null
  windowLabelDisplay: string
  requestCount: number | null
  requestCountDisplay: string
  successCount: number | null
  successRate: number | null
  successRateDisplay: string
  totalTokens: number | null
  totalTokensDisplay: string
  lastUsedAt: string | null
  lastUsedAtDisplay: string
}

export interface KeyUsageAccountModelUsage {
  model: string
  requestCount: number
  requestCountDisplay: string
  successRate: number | null
  successRateDisplay: string
  inputTokens: number | null
  inputTokensDisplay: string
  outputTokens: number | null
  outputTokensDisplay: string
  cachedTokens: number | null
  cachedTokensDisplay: string
  reasoningTokens: number | null
  reasoningTokensDisplay: string
  totalTokens: number | null
  totalTokensDisplay: string
  lastUsedAt: string
  lastUsedAtDisplay: string
}

export interface KeyUsageAccount {
  name: string | null
  email: string | null
  capabilities: {
    fullIdentity: boolean
    personalInfo: boolean
    resetCredits: boolean
    refreshQuota: boolean
    quotaForecast: boolean
  }
  id: string
  identity: string | null
  provider: string
  authenticationKind: string
  planType: string | null
  planTypeDisplay: string
  status: AccountStatus
  quota: KeyUsageAccountQuota
  usage: KeyUsageAccountUsageSummary
}

export interface KeyUsageAccountDetail extends Omit<KeyUsageAccount, 'usage'> {
  usage: KeyUsageAccountUsageSummary & {
    inputTokens: number | null
    inputTokensDisplay: string
    outputTokens: number | null
    outputTokensDisplay: string
    cachedTokens: number | null
    cachedTokensDisplay: string
    reasoningTokens: number | null
    reasoningTokensDisplay: string
    createdTokens: number | null
    createdTokensDisplay: string
    readTokens: number | null
    readTokensDisplay: string
    models: KeyUsageAccountModelUsage[]
  }
}

export interface KeyUsageAccountPage {
  scopeState: KeyUsageAccountScopeState
  items: KeyUsageAccount[]
  currentPage: number
  pageSize: number
  total: number
}

export interface KeyUsageQuery {
  startTime: string
  endTime: string
  model?: string
}

export function getKeyUsageOverview(params: KeyUsageQuery, options: RequestOptions = {}) {
  return request<KeyUsageOverview>({
    url: '/api/key-usage/overview',
    method: 'GET',
    params,
    ...options,
  })
}

export function getKeyUsageVersion(options: RequestOptions = {}) {
  return request<KeyUsageVersion>({
    url: '/api/key-usage/version',
    method: 'GET',
    ...options,
  })
}

export function getKeyUsageConfig(options: RequestOptions = {}) {
  return request<KeyUsageConfig>({
    url: '/api/key-usage/config',
    method: 'GET',
    ...options,
  })
}

export function getKeyUsageRecords(
  params: KeyUsageQuery & { kind: KeyUsageRecordKind, currentPage: number, pageSize: number },
  options: RequestOptions = {},
) {
  return request<KeyUsagePage>({
    url: '/api/key-usage/records',
    method: 'GET',
    params,
    ...options,
  })
}

export function getKeyUsageAccounts(
  params: { currentPage: number, pageSize: number },
  options: RequestOptions = {},
) {
  return request<KeyUsageAccountPage>({
    url: '/api/key-usage/accounts',
    method: 'GET',
    params,
    ...options,
  })
}

export function getKeyUsageAccountDetail(accountId: string, options: RequestOptions = {}) {
  return request<KeyUsageAccountDetail>({
    url: '/api/key-usage/accounts/detail',
    method: 'GET',
    params: { accountId },
    ...options,
  })
}

export function getKeyAccountPersonalInfo(params: { accountId: string }, options: RequestOptions = {}) {
  return request<AccountPersonalInfoResponse>({
    url: '/api/key-usage/accounts/personal-info',
    method: 'GET',
    params,
    ...options,
  })
}

export function getKeyAccountForecast(params: { accountId: string }, options: RequestOptions = {}) {
  return request<AccountQuotaForecastResponse>({
    url: '/api/key-usage/accounts/quota-forecast',
    method: 'GET',
    params,
    ...options,
  })
}

export function getKeyAccountResetCredits(params: { accountId: string }, options: RequestOptions = {}) {
  return request<AccountResetCreditsResponse>({
    url: '/api/key-usage/accounts/reset-credits',
    method: 'GET',
    params,
    ...options,
  })
}

export function consumeKeyAccountResetCredit(data: { accountId: string, creditId?: string, redeemRequestId: string }, options: RequestOptions = {}) {
  return request<AccountResetCreditResultResponse>({
    url: '/api/key-usage/accounts/reset-credits',
    method: 'POST',
    data,
    ...options,
  })
}
export function refreshKeyAccountQuota(accountId: string, options: RequestOptions = {}) {
  return request<KeyUsageAccountDetail>({
    url: '/api/key-usage/accounts/quota/refresh',
    method: 'POST',
    data: { accountId },
    ...options,
  })
}
export function keyAccountAvatarUrl(accountId: string) {
  return `${API_BASE_URL}/api/key-usage/accounts/profile-avatar?${new URLSearchParams({ accountId })}`
}
