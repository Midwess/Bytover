//
//  HomeView.swift
//  BitBridge
//
//  Created by Dang Minh Tien on 18/2/25.
//

import Foundation
import SwiftUI

struct HomeView: View {
    @EnvironmentObject private var core: Core
    @State private var selectedTab: Int = 0
    
    init() {
        UITabBar.appearance().unselectedItemTintColor = Theme.LightViolet.uiColor
            .withAlphaComponent(0.4)
    }
    
    var body: some View {
        TabView(selection: $selectedTab) {
            ShareView()
                .tabItem {
                    ButtonNavigation(icon: ImageAsset.GlobeEmpty.image, icon_selected: ImageAsset.GlobeFull.image, isSelected: selectedTab == 1)
                }
                .tag(1)
            ReceiveView()
                .tabItem {
                    ButtonNavigation(icon: ImageAsset.PuzzelEmpty.image, icon_selected: ImageAsset.PuzzelFull.image, isSelected: selectedTab == 2)
                }
                .tag(2)
            SettingView()
                .tabItem {
                    ButtonNavigation(icon: ImageAsset.SettingEmpty.image, icon_selected: ImageAsset.SettingFull.image, isSelected: selectedTab == 3)
                }
                .tag(3)
        }
        .toolbarBackground(.background, for: .navigationBar)
        .accentColor(Theme.LightViolet.color)
    }
}

#Preview("HomeView") {
    HomeView()
        .environmentObject(CoreMock.withSelectedFileTransfers() as Core)
}
