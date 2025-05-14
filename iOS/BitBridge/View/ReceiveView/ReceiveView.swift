//
//  ReceiveView.swift
//  BitBridge
//
//  Created by Dang Minh Tien on 19/2/25.
//

import SwiftUI
import Foundation
import SharedTypes

struct ReceiveView: View {
    @Environment(\.safeAreaInsets) private var safeAreaInsets
    @Environment(\.screenSize) private var screenSize
    @EnvironmentObject private var core: Core
    
    @State var receiveSessions: [ReceiveSessionViewModel] = []
    @State var isShowingOptionAll = false
    
    var body: some View {
        ZStack {
            StunningBackgroundGradient()
                .opacity(0.4)
            ScrollView {
                LazyVStack(pinnedViews: [.sectionHeaders]) {
                    LogoScene(gltfFileName: "Rocket", logoScale: 1.5)
                        .frame(width: screenSize.width, height: 100)
                        .overlay(Theme.gradientHeading
                            .opacity(0.5)
                            .blur(radius: 15)
                            .frame(width: .infinity, height: screenSize.width / 2)
                        )
                    
                    Text("Your Inbox")
                        .padding(.horizontal, 20)
                        .multilineTextAlignment(.center)
                        .modifier(Heading2())
                    
                    Spacer().frame(height: 10)
                    UpgradePremiumButton()
                    
                    HStack(alignment: .center) {
                        Spacer()
                        Button(action: { isShowingOptionAll = true }) {
                            ImageAsset.More.image
                                .foregroundColor(Theme.PrimaryText.color)
                                .scaleEffect(1.6)
                                .confirmationDialog(
                                    "Options", isPresented: $isShowingOptionAll) {
                                        Button("Select") {}
                                    }
                        }
                        .frame(minWidth: 35, alignment: .trailing)
                    }
                    .padding(.horizontal, SpaceTheme.item.value)
                    
                    ForEach(self.receiveSessions, id: \.self.id) { item in
                        ReceiveSessionHeaderView(session: item)
                            .background(Rectangle()
                                .fill(Theme.BlackBase.color)
                                .blur(radius: 10)
                            )
                            .zIndex(2)
                        
                        ReceiveSessionBodyView(session: item)
                            .zIndex(1)
                        
                        Spacer()
                            .frame(width: 10, height: 20)
                    }
                    .padding(.horizontal, SpaceTheme.screen.value)
                    .padding(.top, SpaceTheme.item.value)
                }
            }
        }
        .background(Theme.BlackBase.color)
        .onReceive(self.core.transfer, perform: { value in
            let receivedSessions = value?.received_sessions ?? [];
            
            if receivedSessions.count != self.receiveSessions.count {
                self.receiveSessions = receivedSessions
            }
        })
    }
}

#Preview {
    ReceiveView()
        .environmentObject(CoreMock.withSelectedFileTransfers() as Core)

}
