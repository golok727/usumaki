import { useState } from 'react';
import { C } from './theme';
import type { Tab } from './types';
import { Sidebar } from './sidebar';
import { Modal } from './modal';
import { DashboardPage } from './pages/dashboardPage';
import { InputsPage } from './pages/inputsPage';
import { LayoutPage } from './pages/layoutPage';
import { StressPage } from './pages/stressPage';
import { EventsPage } from './pages/eventsPage';
import { IssuesPage } from './pages/issuesPage';
import { ImagesPage } from './pages/imagesPage';
import { TimerPage } from './pages/timerPage';
import { ClipboardPage } from './pages/clipboardPage';
import { ShikiPage } from './pages/codeHighlight';

export function App() {
  const [activeTab, setActiveTab] = useState<Tab>('dashboard');
  const [showModal, setShowModal] = useState(false);

  const page = {
    dashboard: <DashboardPage />,
    inputs: <InputsPage />,
    layout: <LayoutPage />,
    stress: <StressPage />,
    events: <EventsPage />,
    issues: <IssuesPage />,
    images: <ImagesPage />,
    timer: <TimerPage />,
    clipboard: <ClipboardPage />,
    shiki: <ShikiPage />,
  }[activeTab];

  return (
    <view
      display="flex"
      flexDir="row"
      w="full"
      h="full"
      bg={C.bg}
      position="relative"
      fontFamily="system-ui, -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Oxygen, Ubuntu, Cantarell, 'Open Sans', 'Helvetica Neue', sans-serif"
    >
      <Sidebar
        w="200px"
        active={activeTab}
        setActive={setActiveTab}
        onOpenModal={() => setShowModal(true)}
      />
      <view flex={1} minW={0} h="full" display="flex" flexDir="col" bg={C.bg}>
        {page}
      </view>
      {showModal && <Modal onClose={() => setShowModal(false)} />}
    </view>
  );
}
