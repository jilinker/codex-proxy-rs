import type { Ref } from 'vue'
import type { KeyUsageAccount, KeyUsageAccountScopeState } from '@/api/modules/key-usage'
import { shallowRef, watch } from 'vue'
import { getKeyUsageAccounts } from '@/api/modules/key-usage'
import { useStablePagedQuery } from '@/composables/useStablePagedQuery'

export function useKeyUsageAccounts(active: Ref<boolean>) {
  const scopeState = shallowRef<KeyUsageAccountScopeState>('available')
  const accounts = useStablePagedQuery({
    initialPageSize: 20,
    load: (pagination, options) => getKeyUsageAccounts(pagination, { ...options, silent: true }),
    onSuccess: result => scopeState.value = result.scopeState,
    onError: () => accounts.items.value = [],
  })
  let loaded = false

  watch(active, (value) => {
    if (!value || loaded)
      return
    loaded = true
    void accounts.execute()
  }, { immediate: true })

  // 刷新前移除旧账号数据 防止授权范围变化后继续展示
  function refresh() {
    accounts.items.value = []
    return accounts.execute()
  }

  function changePage(page: number) {
    accounts.items.value = []
    void accounts.execute(page)
  }

  function changePageSize(size: number) {
    accounts.items.value = []
    accounts.pageSize.value = size
    void accounts.reloadFromStart()
  }

  function updateAccount(account: KeyUsageAccount) {
    accounts.items.value = accounts.items.value.map(item => item.id === account.id ? account : item)
  }

  return { accounts, scopeState, refresh, changePage, changePageSize, updateAccount }
}
