<script setup lang="ts">
import type { AccountIdentityPresentation, AccountQuotaPresentation } from '../accountPresentation'
import { BaseIconButton } from '@codex-proxy/ui'
import { RefreshCw, UserRound } from '@lucide/vue'
import { computed, shallowRef, watch } from 'vue'
import AccountProfileModal from '../AccountProfileModal/index.vue'
import AccountQuotaDetails from './Details.vue'
import AccountResetCredits from './ResetCredits.vue'

const props = withDefaults(defineProps<{
  account: AccountIdentityPresentation & AccountQuotaPresentation
  refreshing: boolean
  personalInfo?: boolean
  resetCredits?: boolean
  refreshQuota?: boolean
}>(), { personalInfo: true, resetCredits: true, refreshQuota: true })
const emit = defineEmits<{
  refreshQuota: [accountId: string]
  quotaReset: [accountId: string]
}>()
const profileOpen = shallowRef(false)
const hasPersonalInfo = computed(() => props.personalInfo !== false && ((props.account.capabilities.profile ?? props.account.capabilities.personalInfo ?? false) || props.account.capabilities.subscription))
watch(hasPersonalInfo, (available) => {
  if (!available)
    profileOpen.value = false
})
</script>

<template>
  <AccountQuotaDetails :account="account">
    <template #actions>
      <div v-if="hasPersonalInfo || (resetCredits !== false && account.capabilities.resetCredits) || (refreshQuota !== false && account.capabilities.quotaRefresh)" class="flex shrink-0 items-center gap-0.5">
        <BaseIconButton
          v-if="hasPersonalInfo"
          label="查看个人信息"
          size="sm"
          variant="ghost"
          :pressed="profileOpen"
          @click="profileOpen = true"
        >
          <UserRound class="size-3.5" />
        </BaseIconButton>
        <AccountResetCredits
          v-if="resetCredits !== false && account.capabilities.resetCredits"
          :account="account"
          @consumed="emit('quotaReset', $event)"
        />
        <BaseIconButton
          v-if="refreshQuota !== false && account.capabilities.quotaRefresh"
          variant="ghost"
          size="sm"
          label="刷新额度"
          :loading="refreshing"
          :disabled="refreshing"
          @click="emit('refreshQuota', account.id)"
        >
          <template #loading>
            <RefreshCw class="size-3.5 animate-spin motion-reduce:animate-none" />
          </template>
          <RefreshCw class="size-3.5" />
        </BaseIconButton>
      </div>
    </template>
  </AccountQuotaDetails>
  <AccountProfileModal
    v-if="hasPersonalInfo"
    v-model="profileOpen"
    :account="account"
  />
</template>
