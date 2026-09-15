<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import { theme } from 'ant-design-vue'

const { t } = useI18n()
const { token } = theme.useToken()
const ratings = [
  { key: 'forgotten', days: 1 },
  { key: 'hard', days: 3 },
  { key: 'good', days: 14 },
  { key: 'mastered', days: 30 },
]
</script>

<template>
  <article class="study-plan-rules">
    <header class="rules-intro">
      <span class="rules-eyebrow">{{ t('studyPlanRules.readonly') }}</span>
      <h2>{{ t('studyPlanRules.title') }}</h2>
      <p>{{ t('studyPlanRules.summary') }}</p>
    </header>

    <section class="rules-section">
      <h3>{{ t('studyPlanRules.initial') }}</h3>
      <ol class="rules-timeline">
        <li v-for="day in [1, 2, 4, 7, 15]" :key="day">
          {{ t('studyPlanRules.day', { days: day }) }}
        </li>
      </ol>
      <p>{{ t('studyPlanRules.initialNote') }}</p>
    </section>

    <section class="rules-section">
      <h3>{{ t('studyPlanRules.assessment') }}</h3>
      <p>{{ t('studyPlanRules.intervalNote') }}</p>
      <table class="rules-table">
        <thead>
          <tr>
            <th>{{ t('studyPlanRules.rating') }}</th>
            <th>{{ t('studyPlanRules.first') }}</th>
            <th>{{ t('studyPlanRules.later') }}</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="rating in ratings" :key="rating.key">
            <th scope="row">{{ t(`studyPlanRules.${rating.key}`) }}</th>
            <td :data-label="t('studyPlanRules.first')">
              {{ t('studyPlanRules.days', { days: rating.days }, rating.days) }}
            </td>
            <td :data-label="t('studyPlanRules.later')">
              {{ t(`studyPlanRules.${rating.key}Rule`) }}
            </td>
          </tr>
        </tbody>
      </table>
    </section>

    <section class="rules-section">
      <h3>{{ t('studyPlanRules.consolidation') }}</h3>
      <ol class="rules-timeline">
        <li v-for="days in [30, 60, 120]" :key="days">
          {{ t('studyPlanRules.days', { days }, days) }}
        </li>
      </ol>
      <p>{{ t('studyPlanRules.consolidationNote') }}</p>
      <p>{{ t('studyPlanRules.continueNote') }}</p>
    </section>

    <section class="rules-section">
      <h3>{{ t('studyPlanRules.example') }}</h3>
      <p>{{ t('studyPlanRules.exampleNote') }}</p>
      <p class="rules-example">3 → 5 → 8 → 12 → 18 → 27 → 30</p>
      <p>{{ t('studyPlanRules.exampleFootnote') }}</p>
    </section>

    <section class="rules-section">
      <h3>{{ t('studyPlanRules.notes') }}</h3>
      <ul class="rules-notes">
        <li v-for="key in ['overdue', 'sameDay', 'history', 'legacy']" :key="key">
          {{ t(`studyPlanRules.${key}`) }}
        </li>
      </ul>
      <p class="rules-disclaimer">{{ t('studyPlanRules.disclaimer') }}</p>
    </section>
  </article>
</template>

<style scoped>
.study-plan-rules {
  max-width: 880px;
  margin: 0 auto;
  padding: 8px 16px 24px;
  color: v-bind('token.colorText');
  line-height: 1.8;
  overflow-wrap: anywhere;
}
.rules-intro {
  margin-bottom: 24px;
}
.rules-eyebrow {
  color: v-bind('token.colorTextSecondary');
  font-size: 12px;
}
h2 {
  margin: 8px 0 12px;
  font-size: 22px;
  font-weight: 700;
}
h3 {
  margin: 0 0 16px;
  font-size: 17px;
  font-weight: 600;
}
p {
  margin: 12px 0 0;
}
.rules-section {
  padding: 24px;
  margin-top: 20px;
  border: 1px solid v-bind('token.colorBorder');
  border-radius: 12px;
  background: v-bind('token.colorBgContainer');
}
.rules-timeline {
  display: flex;
  flex-wrap: wrap;
  gap: 12px;
  padding: 0;
  list-style: none;
}
.rules-timeline li {
  padding: 6px 12px;
  border-radius: 6px;
  background: v-bind('token.colorFillAlter');
  font-weight: 600;
}
.rules-table {
  color: inherit;
  width: 100%;
  margin-top: 16px;
  border-collapse: collapse;
  text-align: left;
}
.rules-table th,
.rules-table td {
  padding: 12px;
  border-bottom: 1px solid v-bind('token.colorBorderSecondary');
  vertical-align: top;
}
.rules-table thead {
  background: v-bind('token.colorFillAlter');
}
.rules-table th {
  font-weight: 600;
}
.rules-table th:first-child {
  min-width: 90px;
}
.rules-example {
  padding: 12px 16px;
  border-radius: 8px;
  background: v-bind('token.colorFillAlter');
  font-size: 18px;
  font-weight: 600;
}
.rules-notes {
  padding-left: 20px;
  margin: 0;
  list-style: disc;
}
.rules-notes li + li {
  margin-top: 12px;
}
.rules-disclaimer {
  color: v-bind('token.colorTextSecondary');
}
@media (max-width: 600px) {
  .study-plan-rules {
    padding: 4px 4px 20px;
  }
  .rules-section {
    padding: 16px;
  }
  .rules-table thead {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip-path: inset(50%);
  }
  .rules-table,
  .rules-table tbody,
  .rules-table tr,
  .rules-table th,
  .rules-table td {
    display: block;
  }
  .rules-table tr {
    padding: 12px 0;
    border-bottom: 1px solid v-bind('token.colorBorderSecondary');
  }
  .rules-table th,
  .rules-table td {
    padding: 4px 0;
    border: 0;
  }
  .rules-table td::before {
    content: attr(data-label) '：';
    color: v-bind('token.colorTextSecondary');
  }
}
</style>
