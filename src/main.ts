import { createApp } from 'vue'
import { createPinia } from 'pinia'
import {
  Button,
  Collapse,
  ConfigProvider,
  Divider,
  Drawer,
  Form,
  Input,
  InputNumber,
  Menu,
  Modal,
  Progress,
  Radio,
  Select,
  Space,
  Spin,
  Switch,
  Tag,
  Tooltip,
} from 'ant-design-vue'
import 'ant-design-vue/dist/reset.css'
import './index.css'
import App from './App.vue'
import i18n from './i18n'

const app = createApp(App)
const pinia = createPinia()

app.use(pinia)
for (const component of [
  Button,
  Collapse,
  ConfigProvider,
  Divider,
  Drawer,
  Form,
  Input,
  InputNumber,
  Menu,
  Modal,
  Progress,
  Radio,
  Select,
  Space,
  Spin,
  Switch,
  Tag,
  Tooltip,
]) {
  app.use(component)
}
app.use(i18n)
app.mount('#app')
