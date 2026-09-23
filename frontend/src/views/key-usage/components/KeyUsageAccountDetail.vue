<script setup lang="ts">
import type { KeyUsageAccountDetail } from '@/api/modules/key-usage'
import { provide, shallowRef, watch } from 'vue'
import { consumeKeyAccountResetCredit, getKeyAccountForecast, getKeyAccountPersonalInfo, getKeyAccountResetCredits, getKeyUsageAccountDetail, keyAccountAvatarUrl, refreshKeyAccountQuota } from '@/api/modules/key-usage'
import { ApiError } from '@/api/request'
import BaseButton from '@/components/base/BaseButton.vue'
import BaseSkeleton from '@/components/base/BaseSkeleton.vue'
import { useRequestState } from '@/composables/useRequestState'
import AccountExpandedPanels from '@/views/accounts/components/AccountExpandedPanels.vue'
import AccountQuotaPanel from '@/views/accounts/components/AccountQuotaPanel/index.vue'
import AccountUsagePanel from '@/views/accounts/components/AccountUsagePanel.vue'
import { accountOperationsKey } from '@/views/accounts/composables/accountOperations'

const props = defineProps<{ accountId: string }>()
const emit = defineEmits<{ updated: [account: KeyUsageAccountDetail], accessChanged: [] }>()
const account = shallowRef<KeyUsageAccountDetail>()
const request = useRequestState()
const { loading, error } = request
const refreshing = shallowRef(false)

async function guarded<T>(promise: Promise<T>): Promise<T> {
  try {
    return await promise
  }
  catch (cause) {
    if (cause instanceof ApiError && [401, 403, 404].includes(cause.status ?? 0)) {
      account.value = undefined
      emit('accessChanged')
      request.error.value = '账号授权已变更 请刷新列表'
    }
    throw cause
  }
}
async function refreshQuota(accountId: string) {
  refreshing.value = true
  const version = request.start()
  try {
    const result = await guarded(refreshKeyAccountQuota(accountId, { signal: request.signal }))
    if (request.isCurrent(version)) {
      account.value = result
      emit('updated', result)
    }
  }
  finally {
    request.finish(version)
    refreshing.value = false
  }
}
provide(accountOperationsKey, {
  scope: 'key',
  personalInfo: (...args) => guarded(getKeyAccountPersonalInfo(...args)),
  forecast: (...args) => guarded(getKeyAccountForecast(...args)),
  resetCredits: (...args) => guarded(getKeyAccountResetCredits(...args)),
  consumeResetCredit: (...args) => guarded(consumeKeyAccountResetCredit(...args)),
  avatarUrl: keyAccountAvatarUrl,
  refreshQuota,
})

// 每个展开行独立请求 卸载后取消并丢弃旧响应
async function load() {
  const id = request.start()
  account.value = undefined
  try {
    const result = await guarded(getKeyUsageAccountDetail(props.accountId, { signal: request.signal, silent: true }))
    if (request.isCurrent(id))
      account.value = result
  }
  catch (cause) {
    request.fail(id, cause)
  }
  finally {
    request.finish(id)
  }
}

watch(() => props.accountId, load, { immediate: true })
</script>

<template>
  <div v-if="loading && !account" class="grid gap-3 p-4 lg:grid-cols-3" aria-label="加载账号详情" aria-busy="true">
    <BaseSkeleton v-for="index in 3" :key="index" class="h-64 rounded-cp-lg" />
  </div>
  <div v-else-if="error" role="alert" class="flex items-center justify-center gap-3 p-6">
    <span class="text-cp-sm text-cp-error-text">{{ error }}</span>
    <BaseButton variant="secondary" @click="load">
      重试
    </BaseButton>
  </div>
  <AccountExpandedPanels v-else-if="account">
    <template #quota>
      <AccountQuotaPanel
        :account="account"
        :refreshing="refreshing"
        :personal-info="account.capabilities.personalInfo"
        :reset-credits="account.capabilities.resetCredits"
        :refresh-quota="account.capabilities.refreshQuota"
        @refresh-quota="refreshQuota($event).catch(() => {})"
        @quota-reset="refreshQuota($event).catch(() => {})"
      />
    </template>
    <template #usage>
      <AccountUsagePanel :account="account" :show-billing="false" :allow-forecast="account.capabilities.quotaForecast" />
    </template>
  </AccountExpandedPanels>
</template>
