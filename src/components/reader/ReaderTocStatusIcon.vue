<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import {
  CalendarOutlined,
  CheckCircleOutlined,
  ClockCircleOutlined,
  CloseCircleOutlined,
  MinusCircleOutlined,
} from '@ant-design/icons-vue'
import type { StudyPlanUnitStatus } from '@/types'

const props = defineProps<{ status: StudyPlanUnitStatus }>()
const { t } = useI18n()
const label = computed(() => t(`reader.studyStatus.${props.status}`))
</script>

<template>
  <span
    class="toc-status-icon inline-flex shrink-0 items-center text-[13px]"
    :class="{
      'text-slate-400 dark:text-slate-500': status === 'unplanned' || status === 'abandoned',
      'text-blue-500 dark:text-blue-400': status === 'scheduled',
      'text-orange-500 dark:text-orange-400': status === 'active',
      'text-green-600 dark:text-green-400': status === 'ended',
    }"
    :title="label"
    :aria-label="label"
  >
    <MinusCircleOutlined v-if="status === 'unplanned'" />
    <CalendarOutlined v-else-if="status === 'scheduled'" />
    <ClockCircleOutlined v-else-if="status === 'active'" />
    <CheckCircleOutlined v-else-if="status === 'ended'" />
    <CloseCircleOutlined v-else />
  </span>
</template>
