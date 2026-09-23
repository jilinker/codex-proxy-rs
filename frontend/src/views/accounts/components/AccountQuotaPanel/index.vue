<script setup lang="ts">
import type { AccountRow } from '../../constants'
import { RefreshCw, UserRound } from '@lucide/vue'
import { shallowRef } from 'vue'
import BaseIconButton from '@/components/base/BaseIconButton.vue'
import AccountProfileModal from '../AccountProfileModal/index.vue'
import AccountQuotaDetails from './Details.vue'
import AccountResetCredits from './ResetCredits.vue'

defineProps<{ account: AccountRow, refreshing: boolean }>()
const emit = defineEmits<{
  refreshQuota: [accountId: string]
  quotaReset: [accountId: string]
}>()
const profileOpen = shallowRef(false)
</script>

<template>
  <AccountQuotaDetails :account="account">
    <template #actions>
      <div v-if="account.authenticationKind !== 'api_key'" class="flex shrink-0 items-center gap-0.5">
        <BaseIconButton
          v-if="account.provider === 'openai' && account.authenticationKind === 'oauth'"
          label="查看个人信息"
          size="sm"
          variant="ghost"
          :pressed="profileOpen"
          @click="profileOpen = true"
        >
          <UserRound class="size-3.5" />
        </BaseIconButton>
        <AccountResetCredits
          v-if="account.provider === 'openai' && account.authenticationKind === 'oauth'"
          :account="account"
          @consumed="emit('quotaReset', $event)"
        />
        <BaseIconButton
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
    v-if="account.provider === 'openai' && account.authenticationKind === 'oauth'"
    v-model="profileOpen"
    :account="account"
  />
</template>
