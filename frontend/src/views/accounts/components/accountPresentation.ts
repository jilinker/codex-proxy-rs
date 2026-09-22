import type { Account, AccountModelUsage, AccountQuota, AccountUsage } from '@/api/modules/accounts'

// 两种身份共用的只读展示数据
export type AccountQuotaPresentation = Pick<Account, 'provider' | 'authenticationKind' | 'planType' | 'planTypeDisplay'> & {
  quota: Pick<AccountQuota, 'windows' | 'refreshedAtDisplay'> & { availability?: string }
}

export type AccountRecentModel = Pick<AccountModelUsage, 'model' | 'lastUsedAt'>

export type AccountSummaryPresentation = Pick<Account, 'authenticationKind'> & {
  quota: Pick<AccountQuota, 'windows'>
  usage: Pick<AccountUsage, 'requestCount' | 'totalTokensDisplay' | 'windowLabelDisplay'> & {
    models?: AccountRecentModel[]
    recentModel?: AccountRecentModel | null
  }
}

export type AccountUsagePresentation = Pick<AccountUsage, 'windowLabelDisplay' | 'totalTokens' | 'totalTokensDisplay' | 'inputTokensDisplay'
  | 'outputTokensDisplay' | 'cachedTokensDisplay' | 'reasoningTokensDisplay' | 'readTokensDisplay'> & {
    costs?: AccountUsage['costs']
    models: (Pick<AccountModelUsage, 'model' | 'requestCountDisplay' | 'successRate' | 'successRateDisplay'
    | 'inputTokensDisplay' | 'outputTokensDisplay' | 'cachedTokensDisplay'
    | 'totalTokensDisplay' | 'lastUsedAtDisplay'> & Partial<Pick<AccountModelUsage, 'billingAmountUsdDisplay'>>)[]
  }
