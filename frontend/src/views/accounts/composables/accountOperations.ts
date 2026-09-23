import type { InjectionKey } from 'vue'
import { inject } from 'vue'
import { accountProfileAvatarUrl, consumeAccountResetCredit, getAccountPersonalInfo, getAccountQuotaForecast, getAccountResetCredits } from '@/api'

export interface AccountOperations {
  scope: 'admin' | 'key'
  personalInfo: typeof getAccountPersonalInfo
  forecast: typeof getAccountQuotaForecast
  resetCredits: typeof getAccountResetCredits
  consumeResetCredit: typeof consumeAccountResetCredit
  avatarUrl: typeof accountProfileAvatarUrl
  refreshQuota?: (accountId: string) => Promise<void>
}

export const accountOperationsKey: InjectionKey<AccountOperations> = Symbol('accountOperations')
const adminOperations: AccountOperations = {
  scope: 'admin',
  personalInfo: getAccountPersonalInfo,
  forecast: getAccountQuotaForecast,
  resetCredits: getAccountResetCredits,
  consumeResetCredit: consumeAccountResetCredit,
  avatarUrl: accountProfileAvatarUrl,
}

export function useAccountOperations() {
  return inject(accountOperationsKey, adminOperations)
}
