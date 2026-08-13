import type { PluginDefinition } from '@gameap/plugin-sdk';

// Pages
import AdminPage from './pages/AdminPage.vue';
import NodeUsersPage from './pages/NodeUsersPage.vue';

// Tabs
import FtpUsersTab from './tabs/FtpUsersTab.vue';

// Translations
import { translations } from './translations';

// Mocks
import { registerMockHandlers } from './mocks/handlers';

export const ftpUsersPlugin: PluginDefinition = {
  id: 'files',
  name: 'FTP Users Manager',
  version: __PLUGIN_VERSION__,
  apiVersion: '1.0',
  description: 'Manage FTP/SFTP users for game servers',
  author: 'GameAP',

  translations,

  onInit() {
    registerMockHandlers();
  },

  routes: [
    {
      path: '/',
      name: 'index',
      component: AdminPage,
      meta: {
        title: 'FTP',
        requiresAuth: true,
        requiresAdmin: true,
      },
    },
    {
      path: '/nodes/:nodeId/users',
      name: 'node-users',
      component: NodeUsersPage,
      meta: {
        title: 'Node Users',
        requiresAuth: true,
        requiresAdmin: true,
      },
    },
  ],

  menuItems: [
    {
      section: 'admin',
      icon: 'ftp',
      text: '@:ftp_users_admin',
      route: { name: 'index' },
      order: 50,
      adminOnly: true,
    },
  ],

  slots: {
    'server-tabs': [
      {
        component: FtpUsersTab,
        order: 60,
        label: '@:ftp_users',
        icon: 'ftp',
        name: 'ftp-users',
        // The backend registers the ftp-users-view/-manage abilities; the
        // panel exposes them as plugin:files:<name> and admins always pass.
        checkPermission: {
          type: 'hasServerPermissions',
          permissions: ['plugin:files:ftp-users-view'],
        },
      },
    ],
  },
};
