//
//  HomeView.swift
//  BitBridge
//
//  Created by Dang Minh Tien on 18/2/25.
//

import Foundation
import SwiftUI

struct HomeView: View {
    @Environment(\.safeAreaInsets) private var safeAreaInsets
    @EnvironmentObject private var core: Core
    @State private var selectedTab: Int = 1
    @State private var previousTab: Int = 1

    init() {
        UITabBar.appearance().unselectedItemTintColor = Theme.LightViolet.uiColor
            .withAlphaComponent(0.4)
    }

    var body: some View {
        TabView(selection: $selectedTab) {
            ShareView()
                .tabItem {
                    ButtonNavigation(icon: ImageAsset.GlobeEmpty.image, icon_selected: ImageAsset.GlobeFull.image, index: 1, selection: $selectedTab)
                        .scaleEffect(1.08)
                }
                .tag(1)
            ReceiveView()
                .tabItem {
                    ButtonNavigation(icon: ImageAsset.MailReceiveEmpty.image, icon_selected: ImageAsset.MailReceiveFull.image, index: 2, selection: $selectedTab)
                }
                .tag(2)
            SettingView()
                .tabItem {
                    ButtonNavigation(icon: ImageAsset.PuzzelEmpty.image, icon_selected: ImageAsset.PuzzelFull.image, index: 3, selection: $selectedTab)
                }
                .tag(3)
            SettingView()
                .tabItem {
                    ButtonNavigation(icon: ImageAsset.SettingEmpty.image, icon_selected: ImageAsset.SettingFull.image, index: 4, selection: $selectedTab)
                }
                .tag(4)
        }
        .animation(.easeInOut, value: selectedTab)
        .accentColor(Theme.LightViolet.color)
    }
}

#Preview("HomeView") {
    HomeView()
        .environmentObject(CoreMock.withSelectedFileTransfers() as Core)
}
