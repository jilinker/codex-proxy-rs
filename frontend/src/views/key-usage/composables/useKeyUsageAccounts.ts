import type { Ref } from 'vue'
import type { KeyUsageAccountDetail, KeyUsageAccountScopeState } from '@/api/modules/key-usage'
import { shallowRef, watch } from 'vue'
import { getKeyUsageAccountDetail, getKeyUsageAccounts } from '@/api/modules/key-usage'
import { useRequestState } from '@/composables/useRequestState'
import { useStablePagedQuery } from '@/composables/useStablePagedQuery'

export function useKeyUsageAccounts(active: Ref<boolean>) {
  const scopeState = shallowRef<KeyUsageAccountScopeState>('available')
  const detail = shallowRef<KeyUsageAccountDetail>()
  const detailOpen = shallowRef(false)
  const detailRequest = useRequestState()
  const accounts = useStablePagedQuery({
    initialPageSize: 20,
    load: (pagination, options) => getKeyUsageAccounts(pagination, { ...options, silent: true }),
    onSuccess: result => scopeState.value = result.scopeState,
  })
  let loaded = false

  watch(active, (value) => {
    if (!value || loaded)
      return
    loaded = true
    void accounts.execute()
  }, { immediate: true })

  async function openDetail(accountId: string) {
    detail.value = undefined
    detailOpen.value = true
    const requestId = detailRequest.start()
    try {
      const result = await getKeyUsageAccountDetail(accountId, {
        signal: detailRequest.signal,
        silent: true,
      })
      if (detailRequest.isCurrent(requestId))
        detail.value = result
    }
    catch (cause) {
      detailRequest.fail(requestId, cause)
    }
    finally {
      detailRequest.finish(requestId)
    }
  }

  function refresh() {
    return accounts.execute(undefined, { silent: accounts.items.value.length > 0 })
  }

  function changePage(page: number) {
    void accounts.execute(page)
  }

  function changePageSize(size: number) {
    accounts.pageSize.value = size
    void accounts.reloadFromStart()
  }

  return {
    accounts,
    scopeState,
    detail,
    detailOpen,
    detailLoading: detailRequest.loading,
    detailError: detailRequest.error,
    refresh,
    openDetail,
    changePage,
    changePageSize,
  }
}
