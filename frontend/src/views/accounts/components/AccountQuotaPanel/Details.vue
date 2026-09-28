<script setup lang="ts">
import type { AccountQuotaPresentation } from '../accountPresentation'
import { BaseEmpty } from '@codex-proxy/ui'
import { computed } from 'vue'
import { formatProviderLabel } from '@/utils/providers'
import { groupedAccountQuotaWindows, orderedPanelQuotaWindows } from '../../constants'
import AccountPlanBadge from '../AccountPlanBadge.vue'
import AccountQuotaPanelEntry from './Entry.vue'

const props = defineProps<{ account: AccountQuotaPresentation }>()
const supportsQuota = computed(() => props.account.capabilities?.quota ?? (props.account.quota.availability !== undefined && props.account.quota.availability !== 'unsupported'))
const quotaEntries = computed(() => groupedAccountQuotaWindows(
  orderedPanelQuotaWindows(props.account.quota.windows),
))
</script>

<template>
  <section class="flex min-h-0 flex-col rounded-lg bg-cp-bg-container p-4 shadow-cp-tertiary">
    <div class="mb-3 flex shrink-0 items-start justify-between gap-3">
      <div class="min-w-0">
        <h3 class="m-0 text-cp-lg font-heavy text-cp-text">
          账号额度
        </h3>
        <p
          v-if="supportsQuota || quotaEntries.length > 0"
          class="m-0 mt-1 flex min-w-0 items-center gap-1.5 text-cp-xs font-emphasis text-cp-text-secondary"
        >
          <span>{{ formatProviderLabel(account.provider) }} 额度</span>
          <template v-if="account.planType">
            <span>·</span>
            <AccountPlanBadge :plan-type="account.planType" :plan-type-display="account.planTypeDisplay" size="sm" />
          </template>
          <span>·</span>
          <span>最近刷新: {{ account.quota.refreshedAtDisplay }}</span>
        </p>
      </div>
      <slot name="actions" />
    </div>

    <div v-if="!supportsQuota && quotaEntries.length === 0" class="grid flex-1 place-items-center">
      <BaseEmpty title="暂不支持查询上游额度" surface="none" />
    </div>
    <div v-else class="grid min-h-0 gap-3">
      <AccountQuotaPanelEntry
        v-for="entry in quotaEntries"
        :key="entry.key"
        :label="entry.label"
        :windows="entry.windows"
      />
      <p v-if="quotaEntries.length === 0" class="m-0 text-cp-sm font-emphasis text-cp-text-secondary">
        {{ account.quota.availability === 'unavailable' ? '额度暂不可用' : '额度待观测' }}
      </p>
    </div>
  </section>
</template>
